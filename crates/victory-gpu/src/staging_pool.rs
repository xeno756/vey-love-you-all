//! (PROJECT ONE) Upload memory for the staging buffers of textures picked
//! by index, cut from a few long-lived upload buffers instead of one
//! committed buffer made per texture.
//!
//! The pass over those textures (`draw::heap_textures`) made a staging
//! buffer for each, used it for one upload and let it go at its list's
//! reset: while a world streams in, 5,071 buffers made in 2.1 s of every
//! 5 s (runs of 25/11), each at least 64 KiB of the PC's memory for 39 KiB
//! of texture on average.
//!
//! A [`Lease`] is a range of a [`Chunk`]. A chunk is handed out again from
//! its start only once every lease of it has ended **and** every submission
//! started before the last of them ended has finished
//! (`crate::host::submissions`, the rule `crate::host::retire` drops objects
//! by): never by a count of frames. While the pool is full, an ask gets a
//! buffer of its own, as before.
//!
//! A range is handed out as zeros, like a new upload buffer: a staging the
//! upload does not write (a texture not mapped yet) must stay black.
//!
//! - `VICTORY_STAGING_POOL=0`: off (a buffer per texture, as before). On by
//!   default since the runs of 25/11, 22:30 to 23:24: a staging buffer made
//!   in 0.026 ms against 0.450, no range written after its lease in 197
//!   chunks handed out again, no poison in 240 s of captured frames.
//! - `VICTORY_STAGING_POOL_MB=<n>`: the most the pool holds (256).
//! - `VICTORY_UPLOAD_POISON=1`: a range is filled with [`POISON`] the
//!   moment its lease ends, and a chunk is checked to hold nothing else
//!   when it is handed out again. A texture uploaded from a range after its
//!   lease ended then shows magenta (BC1 and BC3 blocks of the pattern are
//!   magenta), and a write into one is counted (`broken`).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use windows::Win32::Graphics::Direct3D12::ID3D12Resource;

/// Bytes of a chunk.
pub(crate) const CHUNK: u64 = 8 << 20;
/// Asks above this get a buffer of their own.
pub(crate) const LARGEST: u64 = 2 << 20;
/// Texture data is placed on multiples of this in an upload buffer.
const ALIGN: u64 = 512;
/// What a range holds after its lease, with `VICTORY_UPLOAD_POISON=1`: as a
/// BC1 block (and the colour half of a BC3 block), both endpoints magenta.
pub(crate) const POISON: [u8; 8] = [0x1f, 0xf8, 0x1f, 0xf8, 0, 0, 0, 0];

/// What a new chunk is made of: its upload buffer (none in tests), its
/// mapped memory, and the memory itself where the chunk owns it (tests).
pub(crate) type Made = (Option<ID3D12Resource>, *mut u8, Option<Box<[u8]>>);

pub(crate) struct Chunk {
    buffer: Option<ID3D12Resource>,
    data: *mut u8,
    size: u64,
    _own: Option<Box<[u8]>>,
    /// Bytes handed out from its start; changed under the pool's lock.
    used: AtomicU64,
    /// Leases alive.
    live: AtomicU64,
    /// Submissions started when a lease last ended.
    ended: AtomicU64,
    /// Times it was handed out again from its start.
    reuses: AtomicU64,
}

// SAFETY: the buffer is free-threaded; the memory is written through
// leases of ranges that do not overlap.
unsafe impl Send for Chunk {}
// SAFETY: as above.
unsafe impl Sync for Chunk {}

/// A range of a chunk, until dropped.
pub(crate) struct Lease {
    chunk: Arc<Chunk>,
    base: u64,
    len: u64,
    clock: fn() -> (u64, u64),
    poison: bool,
}

impl Lease {
    /// The upload buffer the range lies in.
    pub(crate) fn buffer(&self) -> Option<&ID3D12Resource> {
        self.chunk.buffer.as_ref()
    }

    /// The range's first byte.
    pub(crate) fn data(&self) -> *mut u8 {
        // SAFETY: within the chunk's mapped memory.
        unsafe { self.chunk.data.add(self.base as usize) }
    }

    /// Where the range begins in its buffer.
    pub(crate) fn base(&self) -> u64 {
        self.base
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        if self.poison {
            // SAFETY: the lease's own range, which nothing else writes.
            unsafe { fill(self.data(), self.len) };
        }
        // The mark before the count: who finds no lease alive finds the
        // mark of the last one.
        self.chunk.ended.fetch_max((self.clock)().0, Ordering::AcqRel);
        self.chunk.live.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Writes [`POISON`] over `len` bytes at `at` (a multiple of 8 into its
/// chunk: ranges begin on [`ALIGN`]).
///
/// # Safety
/// `at` is valid for `len` bytes of writes.
unsafe fn fill(at: *mut u8, len: u64) {
    // SAFETY: the caller's range.
    let bytes = unsafe { std::slice::from_raw_parts_mut(at, len as usize) };
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = POISON[i % POISON.len()];
    }
}

/// Bytes of `len` at `at` that are not [`POISON`].
///
/// # Safety
/// `at` is valid for `len` bytes of reads.
unsafe fn spoiled(at: *const u8, len: u64) -> u64 {
    // SAFETY: the caller's range.
    let bytes = unsafe { std::slice::from_raw_parts(at, len as usize) };
    bytes.iter().enumerate().filter(|(i, b)| **b != POISON[i % POISON.len()]).count() as u64
}

/// Counts since the start: leases given, chunks made, chunks handed out
/// again, asks refused for the pool being full, and bytes found written in
/// ranges whose lease had ended (poison mode).
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Stats {
    pub(crate) leases: u64,
    pub(crate) chunks: u64,
    pub(crate) reuses: u64,
    pub(crate) refused: u64,
    pub(crate) broken: u64,
    /// Leases given from a chunk handed out again (memory an earlier lease
    /// had), and their bytes: what the poison check covers.
    pub(crate) reused_leases: u64,
    pub(crate) reused_bytes: u64,
}

pub(crate) struct Pool {
    chunks: Vec<Arc<Chunk>>,
    current: Option<usize>,
    most: u64,
    clock: fn() -> (u64, u64),
    poison: bool,
    stats: Stats,
}

impl Pool {
    /// A pool of at most `most` bytes. `clock` gives the submissions
    /// started and finished so far.
    pub(crate) fn new(most: u64, clock: fn() -> (u64, u64), poison: bool) -> Pool {
        Pool {
            chunks: Vec::new(),
            current: None,
            most,
            clock,
            poison,
            stats: Stats::default(),
        }
    }

    pub(crate) fn stats(&self) -> Stats {
        self.stats
    }

    /// Chunks handed out again at least once, and chunks in all.
    pub(crate) fn exercised(&self) -> (u64, u64) {
        let again = self.chunks.iter().filter(|c| c.reuses.load(Ordering::Relaxed) > 0).count();
        (again as u64, self.chunks.len() as u64)
    }

    /// Bytes of the chunks made.
    pub(crate) fn bytes(&self) -> u64 {
        self.chunks.iter().map(|c| c.size).sum()
    }

    /// `len` bytes of upload memory, or `None` (too large an ask, or the
    /// pool full with every chunk still read): the caller then makes a
    /// buffer of its own. `make` makes a chunk of the bytes it is given.
    pub(crate) fn lease(&mut self, len: u64, make: impl FnOnce(u64) -> Option<Made>) -> Option<Lease> {
        if len == 0 || len > LARGEST {
            return None;
        }
        let aligned = len.next_multiple_of(ALIGN);
        // The chunk in use, while the ask fits behind what it gave.
        if let Some(i) = self.current {
            let at = self.chunks[i].used.load(Ordering::Relaxed);
            if at + aligned <= self.chunks[i].size {
                return Some(self.take(i, at, aligned, len));
            }
        }
        // Else one no lease is left of, once the submissions started while
        // it had one have finished: from its start again.
        let (_, finished) = (self.clock)();
        let free = self.chunks.iter().position(|c| {
            c.live.load(Ordering::Acquire) == 0 && c.ended.load(Ordering::Acquire) <= finished
        });
        if let Some(i) = free {
            let chunk = &self.chunks[i];
            if self.poison {
                let used = chunk.used.load(Ordering::Relaxed);
                // SAFETY: the chunk's own memory, with no lease alive.
                let broken = unsafe { spoiled(chunk.data, used) };
                if broken > 0 {
                    self.stats.broken += broken;
                    tracing::error!(broken, used, "staging pool: a range was written after its lease ended (VICTORY_UPLOAD_POISON)");
                    // SAFETY: as above.
                    unsafe { fill(chunk.data, used) };
                }
            }
            chunk.used.store(0, Ordering::Relaxed);
            chunk.reuses.fetch_add(1, Ordering::Relaxed);
            self.stats.reuses += 1;
            self.current = Some(i);
            return Some(self.take(i, 0, aligned, len));
        }
        // Else a new one, while the pool may grow.
        if self.bytes() + CHUNK > self.most {
            self.stats.refused += 1;
            return None;
        }
        let (buffer, data, own) = make(CHUNK)?;
        if self.poison {
            // SAFETY: the new chunk's memory.
            unsafe { fill(data, CHUNK) };
        }
        self.chunks.push(Arc::new(Chunk {
            buffer,
            data,
            size: CHUNK,
            _own: own,
            used: AtomicU64::new(0),
            live: AtomicU64::new(0),
            ended: AtomicU64::new(0),
            reuses: AtomicU64::new(0),
        }));
        self.stats.chunks += 1;
        let i = self.chunks.len() - 1;
        self.current = Some(i);
        Some(self.take(i, 0, aligned, len))
    }

    fn take(&mut self, i: usize, at: u64, aligned: u64, len: u64) -> Lease {
        let chunk = &self.chunks[i];
        // A range begins as zeros, as a new upload buffer does: the staging
        // of a texture whose memory the title has not mapped yet is not
        // written at all (`texture.rs`, the partly mapped and all-zero
        // cases) and is copied as it is. Without this it held the bytes of
        // the lease before it, and such a texture showed another's texels
        // instead of black (found on 26/11 by reading; the pool of 25/11).
        // SAFETY: the range `at..at + len` of the chunk, which no lease has.
        unsafe { chunk.data.add(at as usize).write_bytes(0, len as usize) };
        chunk.used.store(at + aligned, Ordering::Relaxed);
        chunk.live.fetch_add(1, Ordering::AcqRel);
        self.stats.leases += 1;
        if chunk.reuses.load(Ordering::Relaxed) > 0 {
            self.stats.reused_leases += 1;
            self.stats.reused_bytes += len;
        }
        Lease {
            chunk: chunk.clone(),
            base: at,
            len,
            clock: self.clock,
            poison: self.poison,
        }
    }
}

static POOL: Mutex<Option<Pool>> = Mutex::new(None);

/// Whether the pool is on (`VICTORY_STAGING_POOL=0`: off).
pub(crate) fn on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_STAGING_POOL").is_none_or(|v| v != "0"))
}

fn poisoned() -> bool {
    std::env::var_os("VICTORY_UPLOAD_POISON").is_some_and(|v| v == "1")
}

thread_local! {
    /// Whether the staging buffers this thread makes now are let go at
    /// their list's reset ([`transient`]).
    static TRANSIENT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `f` with the staging buffers it makes taken from the pool: for the
/// pass that uploads a texture once and lets its staging buffer go.
pub(crate) fn transient<R>(f: impl FnOnce() -> R) -> R {
    let was = TRANSIENT.replace(true);
    let r = f();
    TRANSIENT.set(was);
    r
}

/// `len` bytes from the pool for a staging buffer this thread makes, where
/// the pool is on, the buffer is a transient one and the pool has room.
pub(crate) fn lease(host: &crate::host::Host, len: u64) -> Option<Lease> {
    if !on() || !TRANSIENT.get() {
        return None;
    }
    let mut pool = POOL.lock().unwrap_or_else(|e| e.into_inner());
    let pool = pool.get_or_insert_with(|| {
        let mb = std::env::var("VICTORY_STAGING_POOL_MB").ok().and_then(|v| v.trim().parse::<u64>().ok()).unwrap_or(256);
        let poison = poisoned();
        tracing::info!(mb, poison, "staging pool: transient staging buffers are cut from shared upload buffers (VICTORY_STAGING_POOL)");
        Pool::new(mb << 20, crate::host::submissions, poison)
    });
    pool.lease(len, |bytes| {
        let (buffer, data) = host.upload_buffer(bytes)?;
        crate::host::name(&buffer, || "staging pool chunk".to_owned());
        Some((Some(buffer), data, None))
    })
}

/// For the log: the pool's counts and its bytes (zeros while it is off or
/// unused).
pub(crate) fn stats() -> (Stats, u64, (u64, u64)) {
    let pool = POOL.lock().unwrap_or_else(|e| e.into_inner());
    pool.as_ref().map_or((Stats::default(), 0, (0, 0)), |p| (p.stats(), p.bytes(), p.exercised()))
}

#[cfg(test)]
mod tests {
    use super::*;

    thread_local! {
        /// Submissions started and finished, as the test says.
        static CLOCK: std::cell::Cell<(u64, u64)> = const { std::cell::Cell::new((0, 0)) };
    }

    fn clock() -> (u64, u64) {
        CLOCK.get()
    }

    /// A chunk of plain memory.
    fn memory(bytes: u64) -> Option<Made> {
        let mut own = vec![0u8; bytes as usize].into_boxed_slice();
        let data = own.as_mut_ptr();
        Some((None, data, Some(own)))
    }

    #[test]
    fn leases_of_a_chunk_do_not_overlap_and_begin_on_512() {
        CLOCK.set((0, 0));
        let mut pool = Pool::new(64 << 20, clock, false);
        let a = pool.lease(1000, memory).unwrap();
        let b = pool.lease(513, memory).unwrap();
        let c = pool.lease(1, memory).unwrap();
        assert_eq!((a.base(), b.base(), c.base()), (0, 1024, 2048));
        assert_eq!(pool.stats().chunks, 1);
    }

    #[test]
    fn a_chunk_with_a_lease_alive_is_never_handed_out_again() {
        CLOCK.set((0, 0));
        let mut pool = Pool::new(2 * CHUNK, clock, false);
        // One lease kept, the chunk filled behind it.
        let kept = pool.lease(LARGEST, memory).unwrap();
        let rest: Vec<Lease> = (0..3).map(|_| pool.lease(LARGEST, memory).unwrap()).collect();
        drop(rest);
        // The next ask does not fit: a second chunk, not the first again.
        let next = pool.lease(LARGEST, memory).unwrap();
        assert_eq!(pool.stats(), Stats { leases: 5, chunks: 2, ..Stats::default() });
        assert_ne!(next.data(), kept.data());
        // With both chunks held and the pool at its most: refused.
        let more: Vec<Lease> = (0..3).map(|_| pool.lease(LARGEST, memory).unwrap()).collect();
        assert!(pool.lease(LARGEST, memory).is_none());
        assert_eq!(pool.stats().refused, 1);
        drop((kept, next, more));
    }

    #[test]
    fn a_chunk_waits_for_the_submissions_started_while_it_was_leased() {
        // Submission 7 is running when the leases end.
        CLOCK.set((7, 6));
        let mut pool = Pool::new(CHUNK, clock, false);
        let all: Vec<Lease> = (0..4).map(|_| pool.lease(LARGEST, memory).unwrap()).collect();
        drop(all);
        // No lease is alive, but 7 has not finished: not handed out, and
        // the pool is at its most.
        assert!(pool.lease(LARGEST, memory).is_none());
        assert_eq!(pool.stats().reuses, 0);
        // 7 has finished: the chunk again, from its start.
        CLOCK.set((7, 7));
        let again = pool.lease(LARGEST, memory).unwrap();
        assert_eq!((again.base(), pool.stats().reuses, pool.stats().chunks), (0, 1, 1));
        assert_eq!((pool.stats().reused_leases, pool.stats().reused_bytes, pool.exercised()), (1, LARGEST, (1, 1)));
    }

    #[test]
    fn a_lease_begins_as_zeros_whatever_the_one_before_wrote() {
        CLOCK.set((0, 0));
        for poison in [false, true] {
            let mut pool = Pool::new(CHUNK, clock, poison);
            let all: Vec<Lease> = (0..4).map(|_| pool.lease(LARGEST, memory).unwrap()).collect();
            for lease in &all {
                // SAFETY: the lease's range.
                unsafe { lease.data().write_bytes(0xee, LARGEST as usize) };
            }
            drop(all);
            // The chunk again, from its start: nothing of the texels before.
            let again = pool.lease(4096, memory).unwrap();
            assert_eq!((again.base(), pool.stats().reuses), (0, 1));
            // SAFETY: the lease's range.
            let bytes = unsafe { std::slice::from_raw_parts(again.data(), 4096) };
            assert!(bytes.iter().all(|&b| b == 0), "poison {poison}");
        }
    }

    #[test]
    fn an_ask_too_large_gets_none() {
        CLOCK.set((0, 0));
        let mut pool = Pool::new(64 << 20, clock, false);
        assert!(pool.lease(LARGEST + 1, memory).is_none());
        assert!(pool.lease(0, memory).is_none());
        assert_eq!(pool.stats(), Stats::default());
    }

    #[test]
    fn poison_fills_a_range_at_its_leases_end_and_finds_a_late_write() {
        CLOCK.set((0, 0));
        let mut pool = Pool::new(CHUNK, clock, true);
        let a = pool.lease(4096, memory).unwrap();
        let at = a.data();
        // SAFETY: the lease's range.
        unsafe { at.write_bytes(0x33, 4096) };
        drop(a);
        // SAFETY: the chunk's memory, alive in the pool.
        let after = unsafe { std::slice::from_raw_parts(at, 16) };
        assert_eq!(after, [POISON, POISON].concat());
        // A write after the lease ended (what a late detile would do).
        // SAFETY: as above.
        unsafe { at.add(100).write(0x77) };
        // Fill the chunk so that the next ask takes it from its start.
        let rest: Vec<Lease> = (0..4).map_while(|_| pool.lease(LARGEST, memory)).collect();
        drop(rest);
        let _again = pool.lease(LARGEST, memory).unwrap();
        assert_eq!(pool.stats().broken, 1);
    }
}
