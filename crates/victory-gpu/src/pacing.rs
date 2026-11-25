//! Presents paced by the display.
//!
//! The title paces its frames by our frame clock (`frame.rs`): a tick every
//! interval it set, 33.3 ms for 30 frames a second. That clock is not the
//! display's. Presented with a sync interval of 0, a frame reaches the
//! screen at whichever refresh comes next and takes the place of a frame
//! still waiting: with the two clocks close and each present a millisecond
//! or two early or late, one frame is shown for one refresh, the next for
//! three, and some never. A picture that moves smoothly (the opening
//! film, a pan) then judders as if two films were shown over each other,
//! at a steady "30 frames a second" by the count.
//!
//! So each present asks the display to show its frame for as many
//! refreshes as the title's interval holds: two on a 60 Hz display for 30
//! frames a second, one for 60. Frames are then shown in turn, each for the
//! same time. The number is never more than the interval holds, so the
//! display does not slow the title's own clock down, and it is none (as
//! before) while the title has set no interval or runs faster than the
//! display.
//!
//! `VICTORY_PRESENT_SYNC=0` presents as before; `1` to `4` forces a count.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::HWND;

/// How many refreshes of a display of `hz` a frame is shown for when the
/// title makes one every `frame`: as many as the interval holds (a tenth
/// of a refresh is allowed for a display a little slower than its name,
/// 59.94 Hz for 60), at most four; none when the title has set no interval
/// or makes frames faster than the display shows them.
pub(crate) fn refreshes_for(frame: Option<Duration>, hz: f64) -> u32 {
    let Some(frame) = frame else {
        return 0;
    };
    let held = frame.as_secs_f64() * hz;
    if held < 0.9 {
        return 0;
    }
    ((held + 0.1).floor() as u32).clamp(1, 4)
}

/// The refresh rate of the display `hwnd` is on, asked for again every two
/// seconds (a window can be moved to another display).
fn refresh_hz(hwnd: HWND) -> f64 {
    static KNOWN: Mutex<Option<(Instant, f64)>> = Mutex::new(None);
    let mut known = KNOWN.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, hz)) = *known
        && at.elapsed() < Duration::from_secs(2)
    {
        return hz;
    }
    let hz = display_hz(hwnd).unwrap_or(60.0);
    *known = Some((Instant::now(), hz));
    hz
}

fn display_hz(hwnd: HWND) -> Option<f64> {
    use windows::Win32::Graphics::Gdi::{
        DEVMODEW, ENUM_CURRENT_SETTINGS, EnumDisplaySettingsW, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO,
        MONITORINFOEXW, MonitorFromWindow,
    };
    use windows::core::PCWSTR;
    // SAFETY: plain queries with sized structures of ours.
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
        if !GetMonitorInfoW(monitor, (&raw mut info).cast::<MONITORINFO>()).as_bool() {
            return None;
        }
        let mut mode = DEVMODEW {
            dmSize: size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        if !EnumDisplaySettingsW(PCWSTR(info.szDevice.as_ptr()), ENUM_CURRENT_SETTINGS, &mut mode).as_bool() {
            return None;
        }
        named_hz(mode.dmDisplayFrequency)
    }
}

/// The rate a display mode names, as the display runs it: the modes named
/// 59, 119 and so on are the television rates just under the next number
/// (59.94 Hz); 0 and 1 mean the display's default, which says nothing.
fn named_hz(named: u32) -> Option<f64> {
    match named {
        0 | 1 => None,
        n if (n + 1) % 30 == 0 => Some(f64::from(n + 1) / 1.001),
        n => Some(f64::from(n)),
    }
}

/// What presents did since the last [`take`].
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(crate) struct Presents {
    /// How many.
    pub(crate) count: u32,
    /// Between a present and the one before it, in milliseconds: the sum,
    /// the shortest and the longest.
    pub(crate) between: [f64; 3],
    /// Presents that came less than 0.6 of the title's interval after the
    /// one before (shown over it with no sync interval: a frame lost), and
    /// more than 1.4 of it (the frame before shown again).
    pub(crate) early: u32,
    pub(crate) late: u32,
    /// In the present call itself, in milliseconds: the sum and the longest.
    pub(crate) call: [f64; 2],
    /// The sync interval last used, and the display's rate then.
    pub(crate) sync: u32,
    pub(crate) hz: f64,
    /// The title's own interval in milliseconds (0: none set).
    pub(crate) interval: f64,
}

struct Stats {
    last: Option<Instant>,
    presents: Presents,
}

static STATS: Mutex<Stats> = Mutex::new(Stats {
    last: None,
    presents: Presents {
        count: 0,
        between: [0.0; 3],
        early: 0,
        late: 0,
        call: [0.0; 2],
        sync: 0,
        hz: 0.0,
        interval: 0.0,
    },
});

/// The sync interval for a present to `hwnd`'s window now.
pub(crate) fn sync_interval(hwnd: HWND) -> u32 {
    static FORCED: std::sync::OnceLock<Option<u32>> = std::sync::OnceLock::new();
    let forced = *FORCED.get_or_init(|| {
        let asked = std::env::var("VICTORY_PRESENT_SYNC").ok()?.trim().parse::<u32>().ok()?;
        tracing::info!(sync = asked.min(4), "presents use this sync interval, as asked (VICTORY_PRESENT_SYNC)");
        Some(asked.min(4))
    });
    let hz = refresh_hz(hwnd);
    let interval = crate::frame::interval();
    let sync = forced.unwrap_or_else(|| refreshes_for(interval, hz));
    let mut stats = STATS.lock().unwrap_or_else(|e| e.into_inner());
    // Said when it changes (the first present, another display, another
    // interval set by the title).
    static SAID: Mutex<Option<(u32, u64)>> = Mutex::new(None);
    let mut said = SAID.lock().unwrap_or_else(|e| e.into_inner());
    if said.replace((sync, hz.to_bits())) != Some((sync, hz.to_bits())) {
        tracing::info!(
            sync,
            display_hz = hz,
            title_interval = ?interval,
            "presents: each frame is shown for this many refreshes of the display"
        );
    }
    stats.presents.sync = sync;
    stats.presents.hz = hz;
    stats.presents.interval = interval.map_or(0.0, |i| i.as_secs_f64() * 1e3);
    sync
}

/// A present call that began at `began` has returned.
pub(crate) fn presented(began: Instant) {
    let now = Instant::now();
    let mut stats = STATS.lock().unwrap_or_else(|e| e.into_inner());
    let since = stats.last.replace(began).map(|last| began.duration_since(last).as_secs_f64() * 1e3);
    note(&mut stats.presents, since, now.duration_since(began).as_secs_f64() * 1e3);
}

/// Counts a present that came `since` milliseconds after the one before
/// (none: the first) and whose call took `call` milliseconds.
fn note(p: &mut Presents, since: Option<f64>, call: f64) {
    p.count += 1;
    p.call[0] += call;
    p.call[1] = p.call[1].max(call);
    let Some(since) = since else {
        return;
    };
    p.between[0] += since;
    p.between[1] = if p.between[1] == 0.0 { since } else { p.between[1].min(since) };
    p.between[2] = p.between[2].max(since);
    if p.interval > 0.0 {
        if since < 0.6 * p.interval {
            p.early += 1;
        } else if since > 1.4 * p.interval {
            p.late += 1;
        }
    }
}

/// What presents did since the last call, for the frame account.
pub(crate) fn take() -> Presents {
    let mut stats = STATS.lock().unwrap_or_else(|e| e.into_inner());
    let taken = stats.presents;
    stats.presents = Presents {
        sync: taken.sync,
        hz: taken.hz,
        interval: taken.interval,
        ..Presents::default()
    };
    taken
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame is shown for the refreshes its interval holds, and the
    /// display never slows the title's clock.
    #[test]
    fn a_frame_is_shown_for_the_refreshes_its_interval_holds() {
        let ms = |us: u64| Some(Duration::from_micros(us));
        // 30 frames a second: two refreshes at 60 Hz, also at 59.94.
        assert_eq!(refreshes_for(ms(33_333), 60.0), 2);
        assert_eq!(refreshes_for(ms(33_333), 59.94), 2);
        // 60 frames a second: one.
        assert_eq!(refreshes_for(ms(16_667), 60.0), 1);
        assert_eq!(refreshes_for(ms(16_667), 59.94), 1);
        // Faster displays: as many as fit, never one more (4.8 at 144 Hz).
        assert_eq!(refreshes_for(ms(33_333), 144.0), 4);
        assert_eq!(refreshes_for(ms(33_333), 120.0), 4);
        assert_eq!(refreshes_for(ms(33_333), 75.0), 2);
        assert_eq!(refreshes_for(ms(16_667), 144.0), 2);
        // A title faster than the display, or with no interval: as before.
        assert_eq!(refreshes_for(ms(8_333), 60.0), 0);
        assert_eq!(refreshes_for(None, 60.0), 0);
    }

    #[test]
    fn display_modes_named_59_run_at_59_94() {
        assert_eq!(named_hz(60), Some(60.0));
        assert_eq!(named_hz(144), Some(144.0));
        assert!((named_hz(59).unwrap() - 59.94).abs() < 0.001);
        assert!((named_hz(119).unwrap() - 119.88).abs() < 0.001);
        assert_eq!(named_hz(0), None);
        assert_eq!(named_hz(1), None);
    }

    /// Presents are counted against the title's interval: one that comes
    /// too soon would have been shown over the frame before it, one that
    /// comes too late leaves that frame on the screen.
    #[test]
    fn presents_off_the_titles_interval_are_counted() {
        let mut p = Presents {
            interval: 33.3,
            ..Presents::default()
        };
        note(&mut p, None, 0.5);
        for since in [33.0, 34.0, 12.0, 55.0, 33.3] {
            note(&mut p, Some(since), 1.0);
        }
        assert_eq!((p.count, p.early, p.late), (6, 1, 1));
        assert_eq!((p.between[1], p.between[2]), (12.0, 55.0));
        assert!((p.between[0] - 167.3).abs() < 1e-9);
        assert_eq!(p.call, [5.5, 1.0]);
        // Without an interval nothing is early or late.
        let mut free = Presents::default();
        note(&mut free, Some(1.0), 0.1);
        note(&mut free, Some(500.0), 0.1);
        assert_eq!((free.early, free.late), (0, 0));
    }
}
