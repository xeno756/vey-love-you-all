//! (PROJECT ONE, ported from avatarmanz/VIctory-7900XTX, GPL-3.0-or-later,
//! 4cb90be, its renderer port #1, `crates/victory-gpu/src/port.rs`) The
//! title's 32-bit graphics heap as the card's own memory
//! (`VICTORY_PORT_CONSTANTS=1`, off by default).
//!
//! What the title does (the 1.0.2.1 executable, per the fork): one
//! `XMemVirtualAlloc` of five sizes together (a reservation of 64 KiB pages,
//! graphics memory below 4 GiB, 185 MB), split into five placeholders, each
//! turned into graphics memory of its own protection, then ONE
//! `XMemAllocatePhysicalPages` for all their pages and five
//! `XMemMapPhysicalPages`, in order: `xmem` makes one section of all the
//! pages, so the whole reservation is one stretch of one section. The
//! title's 174 MB buffer (its ring of constants, and what the skinning
//! passes write through one view of all of it) is in it.
//!
//! What the layer did with it: a host copy, the title's writes found and
//! sent up, what shaders wrote snapshotted, compared on the card and fetched
//! back (our run of 26/11 18:10: 11,000 pieces compared a frame, 560 fetched,
//! 36 MB, the copy-back thread 70-90 ms a frame).
//!
//! What is done here instead: the stretch is mapped once more, at an address
//! of ours (a view of the same section: the same physical pages), and opened
//! as a host heap there (`OpenExistingHeapFromAddress1`); title buffers and
//! windows of title memory in it are buffers placed in that heap
//! (`crate::shadow::Shadow`'s in-place form): the card reads and writes the
//! title's bytes where the title wrote them. Nothing is tracked, sent up,
//! fetched back or compared for it. No tile mapping is ever made into this
//! heap (that removes the device).
//!
//! Why our own view: a heap opened from an address holds the pages it was
//! opened on; opened on our view, the title's own views stay free to change.
//! When the title maps other pages there, what is in place is let go
//! (`crate::inplace::title_memory_freeing`) and the region forgotten.
//!
//! What it assumes (the fork's): the CPU steps of ours that read this memory
//! at submission (fills of constant buffers, buffer copies done on the CPU)
//! read what the title's CPU wrote, not what a submission still in the air
//! writes there; how many such steps there are is on the log line.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use windows::Win32::Graphics::Direct3D12::*;
use windows_sys::Win32::System::Memory as mem;

use crate::inplace::{Placed, cover, stretch};

const K64: u64 = 64 << 10;

/// The ported memory lies below this: the title's graphics memory for
/// 32-bit addresses, which only that heap is.
const LOW: u64 = 1 << 32;

/// The most bytes of one region: its pages are locked into the PC's memory
/// while the heap stands.
const MOST: u64 = 512 << 20;

/// `VICTORY_PORT_CONSTANTS=1`: renderer port #1 on.
pub(crate) fn on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("VICTORY_PORT_CONSTANTS").is_ok_and(|v| v.trim() == "1"))
}

/// A stretch of title memory opened as a heap through a view of ours.
struct Region {
    start: u64,
    end: u64,
    heap: ID3D12Heap,
}

// SAFETY: D3D12 heaps are free-threaded.
unsafe impl Send for Region {}

static REGIONS: Mutex<Vec<Region>> = Mutex::new(Vec::new());

/// The lowest start and highest end of every region (both 0: none).
static BOUNDS: [AtomicU64; 2] = [const { AtomicU64::new(0) }; 2];

/// Since the start: regions opened, their bytes; title buffers and windows
/// placed; refused because the memory is not one stretch of a section, not
/// committed, refused by the host, no buffer; regions forgotten. Then, since
/// the last line: fills read out of ported memory on the CPU, their bytes,
/// buffer copies on the CPU that touch it, their bytes.
static STATS: [AtomicU64; 12] = [const { AtomicU64::new(0) }; 12];

/// Whether `[at, at + len)` may be ported: in the title's 32-bit graphics
/// memory.
pub(crate) fn wanted(at: u64, len: u64) -> bool {
    on() && low(at, len)
}

fn low(at: u64, len: u64) -> bool {
    at.checked_add(len).is_some_and(|end| end <= LOW && len > 0)
}

/// Title memory `[start, start + len)` in place, if [`wanted`] and the
/// memory is one stretch of a section: a buffer placed in the region's
/// heap. `None` leaves the caller to do what it did before.
pub(crate) fn place_for(start: u64, len: u64, window: bool) -> Option<Placed> {
    if !wanted(start, len) {
        return None;
    }
    let host = crate::host::host()?;
    match place_in(&host.device, start, len) {
        Ok(placed) => {
            crate::inplace::counted(window, len);
            let n = STATS[2].fetch_add(1, Ordering::Relaxed) + 1;
            if n <= 32 || n.is_power_of_two() {
                tracing::info!(
                    start = format_args!("{start:#x}"),
                    len,
                    window,
                    "renderer port 1: title memory in place, read and written by the card where the title has it"
                );
            }
            Some(placed)
        }
        Err((at, why)) => {
            let n = STATS[at].fetch_add(1, Ordering::Relaxed) + 1;
            if n <= 8 || n.is_power_of_two() {
                tracing::warn!(start = format_args!("{start:#x}"), len, window, why, "renderer port 1: title memory not in place, a copy as before");
            }
            None
        }
    }
}

/// [`place_for`]'s work: the region holding the pages of `[start, start +
/// len)`, opened if none does, and a buffer placed in it. `Err` with the
/// statistic to count and why.
fn place_in(device: &ID3D12Device, start: u64, len: u64) -> Result<Placed, (usize, String)> {
    let at = start & !(K64 - 1);
    let end = (start + len.max(1)).next_multiple_of(K64);
    let mut regions = REGIONS.lock().unwrap_or_else(|e| e.into_inner());
    let i = match regions.iter().position(|r| r.start <= at && end <= r.end) {
        Some(i) => i,
        None => {
            let region = open_region(device, at, end)?;
            regions.push(region);
            let n = regions.len() - 1;
            let r = &regions[n];
            STATS[0].fetch_add(1, Ordering::Relaxed);
            STATS[1].fetch_add(r.end - r.start, Ordering::Relaxed);
            bounds(&regions);
            n
        }
    };
    let region = &regions[i];
    let buffer = crate::inplace::place(device, &region.heap, at - region.start, end - at)
        .map_err(|e| (6, format!("no buffer placed in the region: {e}")))?;
    // SAFETY: a live buffer.
    let gpu = unsafe { buffer.GetGPUVirtualAddress() } + (start - at);
    Ok(Placed { buffer, heap: region.heap.clone(), gpu, bias: start - at })
}

/// Opens the stretch around `[at, end)` (64 KiB aligned) as a heap through
/// a view of ours of the same pages.
fn open_region(device: &ID3D12Device, at: u64, end: u64) -> Result<Region, (usize, String)> {
    let Some((first, bytes, section, offset)) = stretch(at as usize, (end - at) as usize) else {
        return Err((
            3,
            format!(
                "not one stretch of a section: {:?}; views over it: {}",
                cover(at as usize, (end - at) as usize),
                crate::inplace::views_over(at as usize, (end - at) as usize, 24)
            ),
        ));
    };
    // The whole stretch, kept to what may be ported: the next buffer in it
    // finds the region made.
    let (mut first, mut bytes, mut offset) = (first as u64, bytes as u64, offset);
    if first < LOW && first + bytes > LOW {
        bytes = LOW.saturating_sub(first);
    }
    if bytes > MOST {
        // Only what was asked for.
        offset += at - first;
        (first, bytes) = (at, end - at);
    }
    if bytes > MOST {
        return Err((3, format!("{bytes:#x} bytes, over the most of a region")));
    }
    if first > at || first + bytes < end || offset % K64 != 0 {
        return Err((3, format!("the stretch {first:#x}+{bytes:#x} does not hold it")));
    }
    // SAFETY: the section of a view the title has; the runtime keeps its
    // handle open while a view of it is there, and the range is inside it.
    let view = unsafe { mem::MapViewOfFile(section as _, mem::FILE_MAP_WRITE, (offset >> 32) as u32, offset as u32, bytes as usize) }.Value as usize;
    if view == 0 {
        return Err((5, format!("no view of the stretch: {}", std::io::Error::last_os_error())));
    }
    let unmap = || {
        // SAFETY: our own view, nothing opened on it.
        unsafe { mem::UnmapViewOfFile(mem::MEMORY_MAPPED_VIEW_ADDRESS { Value: view as *mut _ }) };
    };
    if !committed(view, bytes as usize) {
        unmap();
        return Err((4, "pages of the stretch are not committed".to_owned()));
    }
    match crate::inplace::open(device, crate::inplace::From::Part(view, bytes as usize)) {
        Ok(heap) => {
            tracing::info!(
                start = format_args!("{first:#x}"),
                bytes,
                section = format_args!("{section:#x}"),
                offset = format_args!("{offset:#x}"),
                view = format_args!("{view:#x}"),
                "renderer port 1: a stretch of the title's graphics memory opened as a host heap through a view of ours"
            );
            // shortcut: the view stays mapped for good (the heap holds its
            // pages while anything names it).
            Ok(Region { start: first, end: first + bytes, heap })
        }
        Err(e) => {
            unmap();
            Err((5, format!("refused by the host: {e}")))
        }
    }
}

/// Whether every page of `[at, at + len)` is committed.
fn committed(at: usize, len: usize) -> bool {
    let mut here = at;
    while here < at + len {
        // SAFETY: plain data filled in by the query.
        let mut info: mem::MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
        // SAFETY: a valid out pointer of the right size.
        if unsafe { mem::VirtualQuery(here as *const _, &mut info, size_of_val(&info)) } == 0 || info.State != mem::MEM_COMMIT {
            return false;
        }
        here = (info.BaseAddress as usize).saturating_add(info.RegionSize);
    }
    true
}

fn bounds(regions: &[Region]) {
    let lo = regions.iter().map(|r| r.start).min().unwrap_or(0);
    let hi = regions.iter().map(|r| r.end).max().unwrap_or(0);
    BOUNDS[0].store(lo, Ordering::Relaxed);
    BOUNDS[1].store(hi, Ordering::Relaxed);
}

/// The title maps other pages at `[at, at + len)`: the regions over it are
/// forgotten (what is placed in them is let go by the caller).
pub(crate) fn forget(at: usize, len: usize) {
    let (at, end) = (at as u64, (at as u64).saturating_add(len as u64));
    if BOUNDS[1].load(Ordering::Relaxed) <= at || BOUNDS[0].load(Ordering::Relaxed) >= end {
        return;
    }
    let mut regions = REGIONS.lock().unwrap_or_else(|e| e.into_inner());
    let before = regions.len();
    regions.retain(|r| r.end <= at || r.start >= end);
    STATS[7].fetch_add((before - regions.len()) as u64, Ordering::Relaxed);
    bounds(&regions);
}

/// Whether `[at, at + len)` touches ported memory (a quick look).
fn touches(at: usize, len: usize) -> bool {
    let hi = BOUNDS[1].load(Ordering::Relaxed);
    hi != 0 && (at as u64) < hi && (at as u64).saturating_add(len as u64) > BOUNDS[0].load(Ordering::Relaxed)
}

/// A fill of upload memory read `len` bytes at `src` on the CPU.
pub(crate) fn filled(src: usize, len: usize) {
    if touches(src, len) {
        STATS[8].fetch_add(1, Ordering::Relaxed);
        STATS[9].fetch_add(len as u64, Ordering::Relaxed);
    }
}

/// For the frame-rate line: what is in place, and what of ours read it on
/// the CPU per frame.
pub(crate) fn log(frames: f64) {
    if !on() {
        return;
    }
    let s: Vec<u64> = STATS.iter().map(|s| s.load(Ordering::Relaxed)).collect();
    let per: Vec<f64> = STATS[8..].iter().map(|s| s.swap(0, Ordering::Relaxed) as f64 / frames).collect();
    tracing::info!(
        "renderer port 1 (VICTORY_PORT_CONSTANTS=1): {} regions of the title's graphics memory in place ({} MB), {} title buffers and windows placed in them \
         (no host copy, nothing sent up or fetched back for them); per frame on the CPU: {:.0} fills read out of them ({:.1} KB); since the start refused: \
         {} not one stretch of a section, {} not committed, {} by the host, {} no buffer; {} regions forgotten (other pages mapped there)",
        s[0],
        s[1] >> 20,
        s[2],
        per[0],
        per[1] / 1024.0,
        s[3],
        s[4],
        s[5],
        s[6],
        s[7],
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only the title's 32-bit memory is asked for.
    #[test]
    fn only_memory_below_4_gib_is_wanted() {
        assert!(low(0x34e1_0000, 0xae0_0000));
        assert!(!low(0xffff_0000, 0x2_0000));
        assert!(!low(0xe_1c80_0000, 0x1000));
        assert!(!low(u64::MAX, 2));
        assert!(!low(0x1000, 0));
    }
}
