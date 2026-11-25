//! `VICTORY_COPY_BACK_ONLY_READBACK=1` (testing, off by default): what
//! shaders write is copied back into title memory only for the buffers the
//! title reads on the CPU.
//!
//! The title (build 1.0.2.1) makes every buffer it reads back on the CPU
//! through one function (its read-back ring, title RVA 0x1d40f90, 22
//! callers: lighting, occlusion, particles, wind, water, autofocus, ...;
//! read statically). A buffer created while that function is on the stack
//! is noted here; with the switch on, shader writes to other memory stay on
//! the card.
//!
//! shortcut: the function is found on the stack by scanning the stack for
//! a return address into it (the title's unwind data is not registered
//! with the system), so a stale return address left in a caller's frame
//! could tag a buffer that is not read back (it is then copied back as
//! before: harmless).

use std::sync::Mutex;
use std::sync::OnceLock;

/// The title's read-back buffer creator (1.0.2.1).
const CREATOR_RVA: u32 = 0x1d40f90;
/// Stack bytes scanned above the creation call.
const SCAN_BYTES: usize = 16 << 10;

/// Whether the switch is on.
pub(crate) fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        let on = std::env::var("VICTORY_COPY_BACK_ONLY_READBACK").is_ok_and(|v| v.trim() == "1");
        if on {
            tracing::warn!(
                "VICTORY_COPY_BACK_ONLY_READBACK: shader writes are copied back only for the title's read-back buffers"
            );
        }
        on
    })
}

/// `[start, end)` of title memory of the read-back buffers alive (let go
/// in [`untag`]: a buffer made later in the same memory need not be one).
static TAGGED: Mutex<crate::memcopy::RangeSet> = Mutex::new(crate::memcopy::RangeSet::new());

/// The creator's code range, once the title image is known: `None` until
/// then, `Some(None)` if it is not this build.
static CREATOR: OnceLock<Option<(usize, usize)>> = OnceLock::new();

/// The `[begin, end)` of the function at `rva` in the image at `base`, from
/// its exception directory.
///
/// # Safety
/// `base` is a mapped PE image.
unsafe fn function_at(base: usize, rva: u32) -> Option<(usize, usize)> {
    // SAFETY: reading the headers of a mapped image.
    unsafe {
        let nt = base + (base as *const u8).add(0x3c).cast::<u32>().read_unaligned() as usize;
        if (nt as *const u32).read_unaligned() != 0x4550 {
            return None;
        }
        // Optional header at +0x18; PE32+ data directories at +0x70; the
        // exception directory is the fourth.
        let dir = nt + 0x18 + 0x70 + 3 * 8;
        let (at, size) = (
            (dir as *const u32).read_unaligned() as usize,
            ((dir + 4) as *const u32).read_unaligned() as usize,
        );
        let entries = std::slice::from_raw_parts((base + at) as *const [u32; 3], size / 12);
        let i = entries.binary_search_by_key(&rva, |e| e[0]).ok()?;
        Some((base + entries[i][0] as usize, base + entries[i][1] as usize))
    }
}

/// The title image holding `address`, if it holds the creator.
fn creator_from(address: usize) -> Option<(usize, usize)> {
    use windows_sys::Win32::System::LibraryLoader::{
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        GetModuleHandleExW,
    };
    let mut module = std::ptr::null_mut();
    // SAFETY: looks up the module holding the address, no reference.
    let found = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            address as *const u16,
            &mut module,
        )
    };
    if found == 0 {
        return None;
    }
    // SAFETY: a module the system knows is a mapped image.
    unsafe { function_at(module as usize, CREATOR_RVA) }
}

/// Called by a resource creation method: whether the title's read-back
/// creator is on the stack. Learns the creator's place from the first
/// caller in the title executable.
pub(crate) fn creating_readback() -> bool {
    use windows_sys::Win32::System::Diagnostics::Debug::RtlCaptureStackBackTrace;
    use windows_sys::Win32::System::Threading::GetCurrentThreadStackLimits;
    // Also with `crate::rebar` placing title buffers (level 2), which must
    // not place these.
    if !enabled() && crate::rebar::level() < 2 {
        return false;
    }
    let creator = match CREATOR.get() {
        Some(c) => *c,
        None => {
            let mut frame = std::ptr::null_mut();
            // SAFETY: room for one frame: the creation method's caller.
            if unsafe { RtlCaptureStackBackTrace(1, 1, &mut frame, std::ptr::null_mut()) } != 1 {
                return false;
            }
            let Some(c) = creator_from(frame as usize) else {
                // A caller outside the executable (a title DLL): ask again later.
                return false;
            };
            tracing::warn!(begin = format_args!("{:#x}", c.0), end = format_args!("{:#x}", c.1), "read-back creator found");
            *CREATOR.get_or_init(|| Some(c))
        }
    };
    let Some((begin, end)) = creator else {
        return false;
    };
    let (mut low, mut high) = (0usize, 0usize);
    // SAFETY: plain out pointers.
    unsafe { GetCurrentThreadStackLimits(&mut low, &mut high) };
    let here = &low as *const usize as usize;
    let top = (here + SCAN_BYTES).min(high);
    let mut at = here & !7;
    while at + 8 <= top {
        // SAFETY: inside this thread's committed stack, above this frame.
        let v = unsafe { (at as *const usize).read() };
        if (begin..end).contains(&v) {
            return true;
        }
        at += 8;
    }
    false
}

/// Notes a read-back buffer at `[start, start + len)`.
pub(crate) fn tag(start: u64, len: u64) {
    let mut t = TAGGED.lock().unwrap_or_else(|e| e.into_inner());
    t.insert(start, start + len);
    let n = t.len();
    if n.is_power_of_two() {
        tracing::warn!(n, start = format_args!("{start:#x}"), len, "read-back buffers tagged");
    }
}

/// Whether a read-back buffer lies in `[start, start + len)`, switch or
/// not (`crate::rebar` keeps such memory where the CPU reads it fast).
pub(crate) fn tagged_in(start: u64, len: u64) -> bool {
    TAGGED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .overlaps(start, start.saturating_add(len))
}

/// Whether shader writes to `[start, start + len)` are copied back: always
/// with the switch off; with it on, only where a read-back buffer lies.
pub(crate) fn copy_back(start: u64, len: u64) -> bool {
    if !enabled() {
        return true;
    }
    let back = TAGGED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .overlaps(start, start.saturating_add(len));
    if back {
        crate::memcopy::add(crate::memcopy::M::BackGoes, 1);
    } else {
        crate::memcopy::add(crate::memcopy::M::BackKept, 1);
        crate::memcopy::add(crate::memcopy::M::BackKeptBytes, len);
    }
    back
}

/// A buffer starting at `start` is let go: if it was a read-back buffer,
/// it is one no more.
pub(crate) fn untag(start: u64) {
    if enabled() {
        TAGGED.lock().unwrap_or_else(|e| e.into_inner()).remove_at(start);
    }
}

/// How many read-back buffers are alive.
pub(crate) fn tagged() -> usize {
    TAGGED.lock().unwrap_or_else(|e| e.into_inner()).len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_test_binary_has_its_own_functions_in_its_table() {
        use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
        // SAFETY: our own executable.
        let base = unsafe { GetModuleHandleW(std::ptr::null()) } as usize;
        let f = function_at as *const () as usize - base;
        // SAFETY: our own mapped image.
        let (b, e) = unsafe { function_at(base, f as u32) }.expect("own function in .pdata");
        assert_eq!(b, function_at as *const () as usize);
        assert!(e > b);
    }
}
