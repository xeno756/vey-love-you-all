//! (PROJECT ONE, ported from avatarmanz/VIctory-7900XTX, GPL-3.0-or-later,
//! its renderer port #2, `crates/victory-gpu/src/port_descriptors.rs`) The
//! title's writes of descriptor records into its shader-visible heap say
//! which slots they wrote, and the per-frame look at every changed heap page
//! goes (`VICTORY_PORT_DESCRIPTORS=1`, off by default; read in [`on`]).
//!
//! What the title does (`victory-loader`'s `descriptor_sites` has the code):
//! its placed shader-visible view heap is the memory of its frame allocator,
//! which hands out the draws' and dispatches' descriptor tables and much else
//! (constants, list nodes). The tables are filled one 32-byte record at a
//! time by six stores in two functions; views made by our calls
//! (`Create*View`, `CopyDescriptors*`) are mirrored where they are made
//! (`descriptor::mirror`). What `descriptor::sync_heaps` did before: the
//! write watch's changed 64 KiB pages of the heap, every slot on them read
//! and compared with the one last seen (run of 26/11 14:56: 254 pages,
//! 519,000 records looked at, 61,000 made again, 110 ms a frame on the thread
//! that submits).
//!
//! What is done here: the six stores are patched to set a byte per slot and
//! per 128 slots in maps of ours ([`MARKS`]), the title's `memcpy` and
//! `memmove` mark the view records they copy into the heap ([`import`]), and
//! our own mirroring marks too (so the record last seen in a slot is the one
//! it shows); `descriptor::sync_heaps` takes the marked slots ([`take`]) in
//! place of the changed pages, and compares and makes only those, as before.
//!
//! Correctness first, the fallback: every [`check_every`] frames the write
//! watch is asked which pages of the heap were written, and every frame a
//! few of those ([`check_budget`]) are looked at whole, as before; a record
//! found changed there that no mark told of is made (so it is right from then
//! on) and counted in the log line.
//!
//! Assumption (the fork's): a slot whose view record the title overwrote with
//! other data (not a view) and no mark told of keeps showing its old view on
//! the host until the fallback looks at its page; nothing reads such a slot
//! as a descriptor (the title wrote no descriptor there).

use std::sync::atomic::{AtomicPtr, AtomicU8, AtomicU64, AtomicUsize, Ordering::{AcqRel, Acquire, Relaxed, Release}};

/// Whether renderer port #2 is on.
pub fn on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("VICTORY_PORT_DESCRIPTORS").is_ok_and(|v| v.trim() == "1"))
}

/// What the patched stores read (`victory_loader::descriptor_sites`,
/// `MARKS_LEN` bytes): the heap's first byte, its length (0: none), the map
/// of slots, the map of chunks of [`CHUNK`] slots.
#[repr(C)]
pub struct Marks {
    start: AtomicU64,
    len: AtomicU64,
    slots: AtomicPtr<AtomicU8>,
    chunks: AtomicPtr<AtomicU8>,
}

const _: () = assert!(size_of::<Marks>() == 32);

pub static MARKS: Marks = Marks {
    start: AtomicU64::new(0),
    len: AtomicU64::new(0),
    slots: AtomicPtr::new(std::ptr::null_mut()),
    chunks: AtomicPtr::new(std::ptr::null_mut()),
};

/// Slots a byte of the map of chunks stands for (4 KiB of records).
const CHUNK: usize = 128;

/// Where the stores' cells are, for `descriptor_sites::apply`.
pub fn marks_address() -> usize {
    std::ptr::from_ref(&MARKS) as usize
}

/// The title's image (base, size) and how many of its stores were patched.
static IMAGE: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];

/// Told by the launcher: `patched` of the title's record stores in the image
/// at `base` (`size` bytes) mark their slots.
pub fn hooked(base: usize, size: usize, patched: usize) {
    IMAGE[0].store(base, Relaxed);
    IMAGE[1].store(size, Relaxed);
    IMAGE[2].store(patched, Relaxed);
}

/// Whether [`take`] stands for the heap's changed pages: the switch is on and
/// the stores were patched.
fn working() -> bool {
    on() && IMAGE[2].load(Relaxed) > 0
}

/// The heap whose slots are marked, and the places in the title's code that
/// last faulted on each of its 64 KiB blocks.
struct Adopted {
    start: u64,
    count: usize,
    slots: &'static [AtomicU8],
    chunks: &'static [AtomicU8],
    writers: &'static [AtomicUsize],
}

static ADOPTED: std::sync::RwLock<Option<Adopted>> = std::sync::RwLock::new(None);

/// A placed shader-visible view heap of `count` records at `start`: the
/// first such is the one whose slots are marked. Whether it is.
pub(crate) fn adopt(start: u64, count: u32) -> bool {
    working() && adopt_now(start, count)
}

fn adopt_now(start: u64, count: u32) -> bool {
    let mut adopted = ADOPTED.write().unwrap_or_else(|e| e.into_inner());
    if adopted.is_some() {
        tracing::info!(start = format_args!("{start:#x}"), count, "renderer port 2: another placed view heap, looked at by its pages as before");
        return false;
    }
    let count = count as usize;
    let leak = |n: usize| -> &'static [AtomicU8] { Box::leak((0..n).map(|_| AtomicU8::new(0)).collect()) };
    // (Never freed: a store of the title's may be marking while the heap
    // goes; a few MB at most.)
    let slots = leak(count);
    let chunks = leak(count.div_ceil(CHUNK));
    let writers: &'static [AtomicUsize] = Box::leak((0..(count * 32).div_ceil(1 << 16)).map(|_| AtomicUsize::new(0)).collect());
    MARKS.slots.store(slots.as_ptr().cast_mut(), Release);
    MARKS.chunks.store(chunks.as_ptr().cast_mut(), Release);
    MARKS.start.store(start, Release);
    // Last: a store that sees a length sees the rest.
    MARKS.len.store(count as u64 * 32, Release);
    *adopted = Some(Adopted { start, count, slots, chunks, writers });
    tracing::info!(
        start = format_args!("{start:#x}"),
        count,
        stores = IMAGE[2].load(Relaxed),
        "renderer port 2: the title's view heap's slots are told by its record stores and our calls, not looked for on its pages (VICTORY_PORT_DESCRIPTORS)"
    );
    true
}

/// The heap at `start` goes: nothing more is marked for it.
pub(crate) fn let_go(start: u64) {
    let mut adopted = ADOPTED.write().unwrap_or_else(|e| e.into_inner());
    if adopted.as_ref().is_some_and(|a| a.start == start) {
        MARKS.len.store(0, Release);
        *adopted = None;
        tracing::info!(start = format_args!("{start:#x}"), "renderer port 2: the marked view heap went");
    }
}

/// Whether the heap at `start` is the one whose slots are marked.
pub(crate) fn adopted(start: u64) -> bool {
    ADOPTED.read().unwrap_or_else(|e| e.into_inner()).as_ref().is_some_and(|a| a.start == start)
}
/// (PROJECT ONE, from avatarmanz/VIctory-7900XTX 4da7c0b, GPL-3.0-or-later.)
/// The title's own `memcpy`, whose calls in the frame allocator's two table
/// copiers go to [`title_copy`] (`victory_loader::descriptor_sites::apply_copies`).
static TITLE_MEMCPY: AtomicUsize = AtomicUsize::new(0);

/// Since the last line: the title's table copies into the marked heap through
/// its own `memcpy`, and the slots they marked.
static TITLE_COPIES: [AtomicU64; 2] = [const { AtomicU64::new(0) }; 2];

/// The function the title's patched `memcpy` calls go to, with the title's
/// `memcpy` at `real` behind it.
pub fn title_copy_hook(real: usize) -> usize {
    TITLE_MEMCPY.store(real, Relaxed);
    title_copy as *const () as usize
}

/// The title's copy of a descriptor table (or of anything else) by its own
/// `memcpy`: done by it, then every slot of the marked heap the copy wrote a
/// byte of is marked, view record or not (a slot the title made null by a
/// copy must be made again too).
unsafe extern "system-unwind" fn title_copy(to: *mut u8, from: *const u8, n: usize) -> *mut u8 {
    // SAFETY: the title's `memcpy`, noted before the title ran.
    let real: Copy = unsafe { std::mem::transmute(TITLE_MEMCPY.load(Relaxed)) };
    // SAFETY: the title's copy, by the function it asked for.
    let out = unsafe { real(to, from, n) };
    let marked = mark_range(to as u64, n as u64);
    if marked > 0 {
        TITLE_COPIES[0].fetch_add(1, Relaxed);
        TITLE_COPIES[1].fetch_add(marked, Relaxed);
    }
    out
}

/// Marks every slot of the marked heap with a byte in `[to, to + n)`: how many.
fn mark_range(to: u64, n: u64) -> u64 {
    let len = MARKS.len.load(Acquire);
    if len == 0 || n == 0 {
        return 0;
    }
    let start = MARKS.start.load(Relaxed);
    let (lo, hi) = (to.max(start), to.saturating_add(n).min(start + len));
    if lo >= hi {
        return 0;
    }
    let (first, last) = ((lo - start) / 32, (hi - 1 - start) / 32);
    for slot in first..=last {
        mark_slot(slot as usize);
    }
    last - first + 1
}

/// Marks the slot at `handle` (a record our own calls wrote), if it is in the
/// marked heap.
#[inline]
pub(crate) fn mark(handle: u64) {
    let len = MARKS.len.load(Acquire);
    if len == 0 {
        return;
    }
    let at = handle.wrapping_sub(MARKS.start.load(Relaxed));
    if at >= len {
        return;
    }
    mark_slot((at / 32) as usize);
    MARKED_BY_US.fetch_add(1, Relaxed);
}

/// Marks slot `slot` of the marked heap (in it: checked by the caller).
#[inline]
fn mark_slot(slot: usize) {
    // SAFETY: the maps are leaked, as long as the length says.
    unsafe {
        (*MARKS.slots.load(Relaxed).add(slot)).store(1, Release);
        (*MARKS.chunks.load(Relaxed).add(slot / CHUNK)).store(1, Release);
    }
}

/// The marked slots of the heap at `start`, each mark cleared, as runs of
/// slots `(first, count)`; `None` if it is not the marked heap.
pub(crate) fn take(start: u64) -> Option<Vec<(u32, u32)>> {
    let began = std::time::Instant::now();
    let adopted = ADOPTED.read().unwrap_or_else(|e| e.into_inner());
    let a = adopted.as_ref().filter(|a| a.start == start)?;
    let mut runs: Vec<(u32, u32)> = Vec::new();
    let mut n = 0u64;
    for (c, chunk) in a.chunks.iter().enumerate() {
        if chunk.load(Relaxed) == 0 {
            continue;
        }
        // Cleared before its slots are read: a mark set meanwhile sets it again.
        chunk.swap(0, AcqRel);
        let first = c * CHUNK;
        for (i, slot) in a.slots[first..(first + CHUNK).min(a.count)].iter().enumerate() {
            if slot.load(Relaxed) == 0 || slot.swap(0, AcqRel) == 0 {
                continue;
            }
            let index = (first + i) as u32;
            n += 1;
            match runs.last_mut() {
                Some((at, len)) if *at + *len == index => *len += 1,
                _ => runs.push((index, 1)),
            }
        }
    }
    TAKEN[0].fetch_add(n, Relaxed);
    TAKEN[1].fetch_add(runs.len() as u64, Relaxed);
    TAKEN[2].fetch_add(began.elapsed().as_nanos() as u64, Relaxed);
    Some(runs)
}

/// A write fault of the title's at `at` (from its instruction at `rip`),
/// if in the marked heap: who wrote its block last, for the log.
pub(crate) fn faulted(at: usize, rip: usize) {
    if MARKS.len.load(Relaxed) == 0 {
        return;
    }
    let Ok(adopted) = ADOPTED.try_read() else { return };
    if let Some(a) = adopted.as_ref() {
        let off = (at as u64).wrapping_sub(a.start);
        if let Some(w) = a.writers.get((off >> 16) as usize) {
            w.store(rip, Relaxed);
        }
    }
}

/// Frames between askings of the write watch for the fallback
/// (`VICTORY_PORT_DESCRIPTORS_EVERY=<frames>`, 8 if not given).
fn check_every() -> u64 {
    static N: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *N.get_or_init(|| std::env::var("VICTORY_PORT_DESCRIPTORS_EVERY").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(8u64).max(1))
}

/// Pages the fallback looks at a frame (`VICTORY_PORT_DESCRIPTORS_CHECK=<pages>`,
/// 16 if not given; 0: none).
fn check_budget() -> usize {
    static N: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *N.get_or_init(|| std::env::var("VICTORY_PORT_DESCRIPTORS_CHECK").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(16usize))
}

/// The fallback's pages (addresses): those the watch said were written, not
/// looked at yet; the frame it last asked in, the frame it last looked in,
/// and the page it goes on from (round the heap).
struct Fallback {
    pages: std::collections::BTreeSet<usize>,
    asked: u64,
    looked: u64,
    next: usize,
}

static FALLBACK: std::sync::Mutex<Fallback> = std::sync::Mutex::new(Fallback { pages: std::collections::BTreeSet::new(), asked: 0, looked: 0, next: 0 });

/// Whether the fallback is whole: every written page, at every sync of the
/// heaps (several a frame), as the layer looked before the port.
fn whole() -> bool {
    check_budget() == usize::MAX && check_every() == 1
}

/// Whether to ask the write watch of the marked heap now (whole: at every
/// sync; else once in [`check_every`] frames).
pub(crate) fn ask_due(frame: u64) -> bool {
    if whole() {
        return true;
    }
    let mut f = FALLBACK.lock().unwrap_or_else(|e| e.into_inner());
    if check_budget() == 0 || f.asked != 0 && frame < f.asked + check_every() {
        return false;
    }
    f.asked = frame.max(1);
    true
}

/// The pages (addresses) the watch said were written, for later.
pub(crate) fn asked(pages: impl IntoIterator<Item = usize>) {
    FALLBACK.lock().unwrap_or_else(|e| e.into_inner()).pages.extend(pages);
}

/// The pages to look at whole this frame (once a frame; none after).
pub(crate) fn to_check(frame: u64) -> Vec<usize> {
    let mut f = FALLBACK.lock().unwrap_or_else(|e| e.into_inner());
    if f.looked == frame {
        return Vec::new();
    }
    f.looked = frame;
    let mut out = Vec::new();
    while out.len() < check_budget() {
        let from = f.next;
        let Some(&page) = f.pages.range(from..).next().or_else(|| f.pages.iter().next()) else {
            break;
        };
        f.pages.remove(&page);
        f.next = page + crate::watch::PAGE;
        out.push(page);
    }
    CHECKED[0].fetch_add(out.len() as u64, Relaxed);
    out
}

/// The fallback looked at the page at `page` whole and made `views` view
/// records and `other` other records that had changed: no mark told of them
/// (or one told since the marks were taken).
pub(crate) fn checked(page: usize, views: u64, other: u64) {
    CHECKED[2].fetch_add(other, Relaxed);
    if views == 0 {
        return;
    }
    CHECKED[1].fetch_add(views, Relaxed);
    let n = MISSED_ALL.fetch_add(views, Relaxed);
    let rip = {
        let adopted = ADOPTED.read().unwrap_or_else(|e| e.into_inner());
        adopted
            .as_ref()
            .and_then(|a| a.writers.get(((page as u64).wrapping_sub(a.start) >> 16) as usize))
            .map_or(0, |w| w.load(Relaxed))
    };
    let (base, size) = (IMAGE[0].load(Relaxed), IMAGE[1].load(Relaxed));
    let writer = if rip >= base && rip < base + size {
        format!("title+{:#x}", rip - base)
    } else if rip != 0 {
        format!("{rip:#x} (not the title's)")
    } else {
        "not known (no fault on its block)".into()
    };
    *WRITERS.lock().unwrap_or_else(|e| e.into_inner()).entry(writer.clone()).or_default() += views;
    if n < 20 || n.is_power_of_two() {
        tracing::warn!(
            page = format_args!("{page:#x}"),
            views,
            %writer,
            "renderer port 2: view records in the title's heap that no store told of (found by the fallback; made now)"
        );
    }
}

/// The runtime's own `memcpy` and `memmove` ([`import`]).
static REAL: [AtomicUsize; 2] = [const { AtomicUsize::new(0) }; 2];

/// Since the last line: the title's copies into the marked heap, records they marked.
static COPIES: [AtomicU64; 2] = [const { AtomicU64::new(0) }; 2];

/// What the title's import of `name` (bound to `address`) is bound to: with
/// the switch, its `memcpy` and `memmove` mark the view records they copy
/// into the heap (the fork's fallback found ~250 view records a frame on 16
/// pages that the six stores had not told of, the runtime's `memcpy` among
/// the last writers of their blocks).
pub fn import(name: &str, address: usize) -> usize {
    let (which, ours) = match name {
        "memcpy" => (0, copy as *const () as usize),
        "memmove" => (1, shift as *const () as usize),
        _ => return address,
    };
    if !on() {
        return address;
    }
    REAL[which].store(address, Relaxed);
    ours
}

type Copy = unsafe extern "system-unwind" fn(*mut u8, *const u8, usize) -> *mut u8;

unsafe extern "system-unwind" fn copy(to: *mut u8, from: *const u8, n: usize) -> *mut u8 {
    // SAFETY: the title's copy, by the function it asked for.
    unsafe { copied(to, from, n, REAL[0].load(Relaxed)) }
}

unsafe extern "system-unwind" fn shift(to: *mut u8, from: *const u8, n: usize) -> *mut u8 {
    // SAFETY: as above.
    unsafe { copied(to, from, n, REAL[1].load(Relaxed)) }
}

/// # Safety
/// `real` copies as `memcpy` does; the rest are the title's.
#[inline]
unsafe fn copied(to: *mut u8, from: *const u8, n: usize, real: usize) -> *mut u8 {
    // SAFETY: the host's function, noted by `import` before the title ran.
    let real: Copy = unsafe { std::mem::transmute(real) };
    // SAFETY: by the caller.
    let out = unsafe { real(to, from, n) };
    let len = MARKS.len.load(Acquire);
    if len != 0 && n >= 4 {
        let start = MARKS.start.load(Relaxed);
        let (lo, hi) = ((to as u64).max(start), (to as u64 + n as u64).min(start + len));
        if lo < hi {
            COPIES[0].fetch_add(1, Relaxed);
            // Each slot whose first word now lies in the copy and is a view's.
            let mut slot = (lo - start).div_ceil(32);
            let mut marked = 0;
            while start + slot * 32 + 4 <= hi {
                let at = start + slot * 32;
                // SAFETY: just written by the copy, inside the heap.
                if unsafe { (at as *const u32).read_unaligned() } == crate::descriptor::VIEW_TAG {
                    mark_slot(slot as usize);
                    marked += 1;
                }
                slot += 1;
            }
            COPIES[1].fetch_add(marked, Relaxed);
        }
    }
    out
}

/// Our own calls' marks since the last line.
static MARKED_BY_US: AtomicU64 = AtomicU64::new(0);
/// Since the last line: slots taken, runs, nanoseconds taking.
static TAKEN: [AtomicU64; 3] = [const { AtomicU64::new(0) }; 3];
/// Since the last line, of the fallback: pages looked at whole, view records
/// no mark told of, other records changed (not views).
static CHECKED: [AtomicU64; 3] = [const { AtomicU64::new(0) }; 3];
/// Since the start: view records the fallback found that no mark told of.
static MISSED_ALL: AtomicU64 = AtomicU64::new(0);
/// Since the start: the missed records by the place that last faulted on their block.
static WRITERS: std::sync::Mutex<std::collections::BTreeMap<String, u64>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// With the frame line: what port #2 did per frame over `frames`.
pub(crate) fn line(frames: f64) {
    if !on() {
        return;
    }
    let [taken, runs, ns] = [0, 1, 2].map(|i| TAKEN[i].swap(0, Relaxed));
    let ours = MARKED_BY_US.swap(0, Relaxed);
    let [pages, missed, other] = [0, 1, 2].map(|i| CHECKED[i].swap(0, Relaxed));
    let copies = [0, 1].map(|i| COPIES[i].swap(0, Relaxed));
    let title_copies = [0, 1].map(|i| TITLE_COPIES[i].swap(0, Relaxed));
    if !working() {
        tracing::info!("renderer port 2 (VICTORY_PORT_DESCRIPTORS=1): not working, no store of the title's was patched");
        return;
    }
    let writers = {
        let by = WRITERS.lock().unwrap_or_else(|e| e.into_inner());
        let mut v: Vec<(&String, &u64)> = by.iter().collect();
        v.sort_by(|a, b| b.1.cmp(a.1));
        v.iter().take(6).map(|(w, n)| format!("{w} {n}")).collect::<Vec<_>>().join(", ")
    };
    tracing::info!(
        "renderer port 2 (VICTORY_PORT_DESCRIPTORS=1) per frame: {:.0} slots told ({:.0} marks by our calls, the rest by the title's {} patched stores and its copies) in {:.0} runs, {:.3} ms taking; \
         the title's memcpy/memmove calls into the heap {:.0}, records they marked {:.0}; its own memcpy's table copies {:.0}, slots they marked {:.0}; fallback: {:.1} pages looked at whole, {:.2} view records no mark told of, {:.0} other records changed; \
         since the start {} view records missed, their writers: {}",
        taken as f64 / frames,
        ours as f64 / frames,
        IMAGE[2].load(Relaxed),
        runs as f64 / frames,
        ns as f64 / 1e6 / frames,
        copies[0] as f64 / frames,
        copies[1] as f64 / frames,
        title_copies[0] as f64 / frames,
        title_copies[1] as f64 / frames,
        pages as f64 / frames,
        missed as f64 / frames,
        other as f64 / frames,
        MISSED_ALL.load(Relaxed),
        if writers.is_empty() { "none".into() } else { writers }
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_layout_is_what_the_stores_read() {
        // `victory_loader::descriptor_sites::MARKS_LEN` (no dependency on the loader).
        assert_eq!(size_of::<Marks>(), 32);
        assert_eq!(std::mem::offset_of!(Marks, start), 0);
        assert_eq!(std::mem::offset_of!(Marks, len), 8);
        assert_eq!(std::mem::offset_of!(Marks, slots), 16);
        assert_eq!(std::mem::offset_of!(Marks, chunks), 24);
    }

    /// Slots marked as the stores mark them, and by our calls, come out once
    /// each, as runs; outside the heap nothing is marked; the fallback's
    /// pages come out a few a frame, once a frame.
    #[test]
    fn marked_slots_are_taken_once_as_runs() {
        let start = 0x1000_0000u64;
        assert!(adopt_now(start, 1000));
        assert!(adopted(start) && !adopted(start + 32));
        let store = |slot: usize| {
            let a = ADOPTED.read().unwrap();
            let a = a.as_ref().unwrap();
            a.slots[slot].store(1, Release);
            a.chunks[slot / CHUNK].store(1, Release);
        };
        store(5);
        mark(start + 6 * 32 + 7);
        mark(start + 300 * 32);
        store(999);
        mark(start - 32);
        mark(start + 1000 * 32);
        assert_eq!(take(start).unwrap(), [(5, 2), (300, 1), (999, 1)]);
        assert_eq!(take(start).unwrap(), []);
        assert!(take(start + 32).is_none());
        let_go(start);
        mark(start + 5 * 32);
        assert!(take(start).is_none());
        assert_eq!(MARKS.len.load(Relaxed), 0);

        let page = crate::watch::PAGE;
        asked((0..20).map(|i| 0x2000_0000 + i * page));
        let first = to_check(7);
        assert_eq!(first.len(), check_budget().min(20));
        assert!(to_check(7).is_empty(), "once a frame");
        let second = to_check(8);
        assert_eq!(first.len() + second.len(), 20);
        assert!(second.iter().all(|p| !first.contains(p)));
    }
}
