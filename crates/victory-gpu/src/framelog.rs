//! The frame log (PROJECT ONE): what each presented frame cost, kept frame
//! by frame where the lines of [`crate::host`]'s meter are averages of a
//! second or more.
//!
//! - The times of the last [`WINDOW`] frames, for their median and their
//!   95th and 99th percentiles: in the window's caption every second and
//!   in the log every few seconds.
//! - `VICTORY_FRAME_CSV=<file>`: one line per frame ([`HEADER`]), for a
//!   run to be compared with another. Without it nothing is written.
//!
//! The counters are the meter's own; it takes them (to zero) once a second
//! and tells so ([`counters_taken`]), a frame being the difference between
//! two readings.

use std::io::Write;
use std::sync::Mutex;
use std::time::Instant;

/// Frames whose times the percentiles are of.
pub(crate) const WINDOW: usize = 300;

/// The columns of the CSV file.
pub(crate) const HEADER: &str = "frame,t_ms,frame_ms,cpu_ms,gpu_ms,back_ms,draws,dropped,indirect_dropped,pipelines_making,\
tex_uploaded,tex_uploaded_mb,tex_waiting,no_upload_run,no_upload_longest,stream_stalls,tex_past_pool,pool_mb,pool_held_mb,\
tex_levels_left_out,card_mb,card_budget_mb,shared_mb,\
pipe_wait_ms,pipes_made,pipe_make_ms,shaders_translated,shader_cache_hits,pipe_oldest_ms,\
shared_budget_mb,tex_due,tex_away,host_tex_alive,host_tex_mb,staging_alive,staging_mb,title_copies_mb,\
windows_alive,windows_mb,ram_free_mb,commit_left_mb";

/// What the meter counts, as read at a frame: microseconds preparing
/// submissions, waiting for the card and copying back; draws run; draws
/// dropped and indirect executions dropped.
pub(crate) type Counters = [u64; 6];
/// Which of [`Counters`] the meter takes to zero every second.
const TAKEN: [bool; 6] = [true, true, true, false, true, true];

struct Log {
    began: Instant,
    last: Instant,
    frame: u64,
    before: Counters,
    times: std::collections::VecDeque<f32>,
    file: Option<std::io::BufWriter<std::fs::File>>,
    memory: Option<[u64; 4]>,
    /// (PROJECT ONE) Pipeline and translation totals at the last frame:
    /// waited µs, made, making µs, translated, found in the cache.
    pipes_before: [u64; 5],
    /// (PROJECT ONE) Windows of title memory outside buffers alive and
    /// their bytes, read now and then (a walk under their lock).
    windows: [u64; 2],
}

/// (PROJECT ONE) The PC's physical memory free and what is left to commit,
/// in bytes.
#[cfg(windows)]
fn system_memory() -> [u64; 2] {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    // SAFETY: plain data the call fills in.
    let mut m: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    m.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
    // SAFETY: a sized structure.
    unsafe { GlobalMemoryStatusEx(&mut m) };
    [m.ullAvailPhys, m.ullAvailPageFile]
}

/// (PROJECT ONE) Pipeline and translation totals as they stand.
#[cfg(windows)]
fn pipe_totals() -> ([u64; 5], u64) {
    let [waited, made, making, oldest] = crate::pipeline::make_stats();
    let (hits, misses) = crate::shader_cache::counts();
    ([waited, made, making, misses, hits], oldest)
}

static LOG: Mutex<Option<Log>> = Mutex::new(None);

fn open() -> Option<std::io::BufWriter<std::fs::File>> {
    let path = std::env::var_os("VICTORY_FRAME_CSV").filter(|p| !p.is_empty())?;
    match std::fs::File::create(&path) {
        Ok(file) => {
            let mut file = std::io::BufWriter::new(file);
            let _ = writeln!(file, "{HEADER}");
            tracing::info!(path = %std::path::Path::new(&path).display(), "frame log: one line per frame (VICTORY_FRAME_CSV)");
            Some(file)
        }
        Err(e) => {
            tracing::warn!(error = %e, "frame log: file not made (VICTORY_FRAME_CSV)");
            None
        }
    }
}

/// A frame was presented: `now` is the meter's counters as they stand,
/// `making` the pipelines being made.
#[cfg(windows)]
pub(crate) fn frame(now: Counters, making: u64) {
    let mut guard = LOG.lock().unwrap_or_else(|e| e.into_inner());
    let at = Instant::now();
    let log = guard.get_or_insert_with(|| Log {
        began: at,
        last: at,
        frame: 0,
        before: now,
        times: std::collections::VecDeque::with_capacity(WINDOW),
        file: open(),
        memory: None,
        pipes_before: pipe_totals().0,
        windows: [0; 2],
    });
    log.frame += 1;
    let ms = at.duration_since(log.last).as_secs_f64() * 1000.0;
    log.last = at;
    if log.frame == 1 {
        // No frame before it to be timed from.
        return;
    }
    if log.times.len() == WINDOW {
        log.times.pop_front();
    }
    log.times.push_back(ms as f32);
    let spent = since(&log.before, &now);
    log.before = now;
    // The card's memory: asked every frame for the file, else now and then
    // for the caption.
    if log.file.is_some() || log.frame % 16 == 2 {
        log.memory = crate::host::host().and_then(|h| h.video_memory());
    }
    let Some(file) = log.file.as_mut() else { return };
    let stream = crate::draw::stream_stats();
    let [budget, used, shared_budget, shared] = log.memory.unwrap_or_default();
    if log.frame % 16 == 2 {
        log.windows = crate::shadow::windows_live();
    }
    let [windows, window_bytes] = log.windows;
    let queue = [1, 2].map(|i| crate::draw::HEAP_TEXTURES_QUEUE[i].load(std::sync::atomic::Ordering::Relaxed));
    let host_textures = [0, 1].map(|i| crate::resource::HOST_TEXTURES[i].load(std::sync::atomic::Ordering::Relaxed));
    let staged = crate::texture::staged_now();
    let [_, copies_bytes] = crate::shadow::live();
    let [ram_free, commit_left] = system_memory();
    let mb = |bytes: u64| bytes >> 20;
    let (pipes, oldest) = pipe_totals();
    let p: Vec<u64> = pipes.iter().zip(log.pipes_before).map(|(now, was)| now.saturating_sub(was)).collect();
    log.pipes_before = pipes;
    let _ = writeln!(
        file,
        "{},{:.1},{ms:.2},{:.2},{:.2},{:.2},{},{},{},{making},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.2},{},{:.2},{},{},{oldest},{},{},{},{},{},{},{},{},{windows},{},{},{}",
        log.frame,
        at.duration_since(log.began).as_secs_f64() * 1000.0,
        spent[0] as f64 / 1000.0,
        spent[1] as f64 / 1000.0,
        spent[2] as f64 / 1000.0,
        spent[3],
        spent[4],
        spent[5],
        stream[0],
        mb(stream[1]),
        stream[2],
        stream[3],
        stream[4],
        stream[5],
        stream[6],
        mb(stream[7]),
        mb(stream[8]),
        stream[9],
        mb(used),
        mb(budget),
        mb(shared),
        p[0] as f64 / 1000.0,
        p[1],
        p[2] as f64 / 1000.0,
        p[3],
        p[4],
        mb(shared_budget),
        queue[0],
        queue[1],
        host_textures[0],
        mb(host_textures[1]),
        staged[0],
        mb(staged[1]),
        mb(copies_bytes),
        mb(window_bytes),
        mb(ram_free),
        mb(commit_left),
    );
    // A run ended by the window's close writes no last line of its own.
    if log.frame.is_multiple_of(30) || ms > 250.0 {
        let _ = file.flush();
    }
}

/// The meter took its counters to zero: the next frame is counted from
/// there.
pub(crate) fn counters_taken() {
    if let Some(log) = LOG.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        for (before, taken) in log.before.iter_mut().zip(TAKEN) {
            if taken {
                *before = 0;
            }
        }
    }
}

/// What `now` counts more than `before`. A counter under what it was has
/// been taken to zero in between and counts from there.
fn since(before: &Counters, now: &Counters) -> Counters {
    std::array::from_fn(|i| if now[i] >= before[i] { now[i] - before[i] } else { now[i] })
}

/// The value under which the share `p` (0 to 1) of `sorted` lies.
fn percentile(sorted: &[f32], p: f64) -> f32 {
    match sorted.len() {
        0 => 0.0,
        n => sorted[((n as f64 * p) as usize).min(n - 1)],
    }
}

/// The median and the 95th and 99th percentiles of the last frames' times
/// (ms), and how many frames they are of.
pub(crate) fn percentiles() -> Option<([f32; 3], usize)> {
    let guard = LOG.lock().unwrap_or_else(|e| e.into_inner());
    let log = guard.as_ref()?;
    if log.times.is_empty() {
        return None;
    }
    let mut sorted: Vec<f32> = log.times.iter().copied().collect();
    sorted.sort_by(f32::total_cmp);
    Some(([0.5, 0.95, 0.99].map(|p| percentile(&sorted, p)), sorted.len()))
}

/// For the window's caption, after the frame rate: the frame times'
/// median and 99th percentile, the card's memory against its budget, the
/// textures waiting, and a stall of their queue if there is one.
#[cfg(windows)]
pub(crate) fn caption() -> String {
    use std::fmt::Write;
    let mut text = String::new();
    if let Some(([p50, _, p99], _)) = percentiles() {
        let _ = write!(text, " | p50 {p50:.0} p99 {p99:.0} ms");
    }
    let memory = LOG.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|l| l.memory);
    if let Some([budget, used, ..]) = memory {
        let _ = write!(text, " | card {}/{} MB", used >> 20, budget >> 20);
    }
    let stream = crate::draw::stream_stats();
    let _ = write!(text, " | {} textures waiting", stream[2]);
    if stream[3] >= 30 {
        text.push_str(" | STREAM STALL");
    }
    text
}

/// The log's line of the frame times and of the texture queue's progress.
#[cfg(windows)]
pub(crate) fn line() {
    let Some(([p50, p95, p99], frames)) = percentiles() else { return };
    let s = crate::draw::stream_stats();
    tracing::info!(
        "frame times of the last {frames} frames: median {p50:.1} ms, 95 in 100 under {p95:.1}, 99 in 100 under {p99:.1}; \
         textures picked by index: a pool of {} MB holds {} MB, new copies leave out {} levels more than asked at most \
         (VICTORY_STREAM_POOL=0: the rule before), {} made past the pool so far; frames without an upload while slots \
         waited for room: {} in a row now, {} at most, {} stalls of 30",
        s[7] >> 20,
        s[8] >> 20,
        s[9],
        s[6],
        s[3],
        s[4],
        s[5]
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_is_the_difference_of_two_readings_even_across_a_take() {
        let before = [900, 5000, 300, 10_000, 4, 2];
        // nothing taken in between
        assert_eq!(since(&before, &[1000, 5600, 300, 10_050, 4, 3]), [100, 600, 0, 50, 0, 1]);
        // the meter took the taken ones to zero (`counters_taken` then says so; this is the
        // reading of a frame that came before the telling)
        assert_eq!(since(&before, &[40, 120, 0, 10_090, 0, 0]), [40, 120, 0, 90, 0, 0]);
        // told: counted from zero, also where the new count passes the old one
        let mut told = before;
        for (b, taken) in told.iter_mut().zip(TAKEN) {
            if taken {
                *b = 0;
            }
        }
        assert_eq!(since(&told, &[2000, 9000, 700, 10_090, 9, 9]), [2000, 9000, 700, 90, 9, 9]);
    }

    #[test]
    fn percentiles_of_frame_times() {
        let sorted: Vec<f32> = (1..=100).map(|i| i as f32).collect();
        assert_eq!(percentile(&sorted, 0.5), 51.0);
        assert_eq!(percentile(&sorted, 0.95), 96.0);
        assert_eq!(percentile(&sorted, 0.99), 100.0);
        assert_eq!(percentile(&sorted, 1.0), 100.0, "never past the last");
        assert_eq!(percentile(&[33.3], 0.99), 33.3);
        assert_eq!(percentile(&[], 0.5), 0.0);
    }

    #[test]
    fn the_file_has_a_column_for_every_value_written() {
        // 41 columns: the line `frame` writes has as many values.
        assert_eq!(HEADER.split(',').count(), 41);
        // One line: a line break in it made three header lines (25/11).
        assert!(!HEADER.contains('\n') && !HEADER.contains(' '));
    }
}
