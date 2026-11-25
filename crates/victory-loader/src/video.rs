//! What the title's video player does, counted.
//!
//! Titles ship their video library (Bink 2) and call it themselves: it
//! decodes on the title's threads into the title's memory, and nothing of
//! it passes through our services. So whether a film that judders is
//! decoded late, skipped, or played twice at once could not be told from
//! a log. The title's calls of a few of the library's functions pass
//! through a counter here on their way; nothing of the library is changed
//! and each call reaches it with the arguments it was made with.
//!
//! Every two seconds of playback a line says how many players are open,
//! how many frames were shown and how many the library asked to skip for
//! lateness, and how long decoding a frame took.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::{Duration, Instant};

use victory_pe::ImportSymbol;

/// How the library's file name begins, whatever the console it was built
/// for (compared without case).
const LIBRARY: &str = "bink2";

const OPEN: usize = 0;
const CLOSE: usize = 1;
const DECODE_ASYNC: usize = 2;
const DECODE_WAIT: usize = 3;
const DECODE: usize = 4;
const NEXT_FRAME: usize = 5;
const SHOULD_SKIP: usize = 6;

/// The functions watched: every one takes at most four arguments, all
/// integers or pointers, and returns an integer, a pointer or nothing.
const WATCHED: [(&str, Call); 7] = [
    ("BinkOpenWithOptions", open),
    ("BinkClose", close),
    ("BinkDoFrameAsyncMulti", decode_async),
    ("BinkDoFrameAsyncWait", decode_wait),
    ("BinkDoFrame", decode),
    ("BinkNextFrame", next_frame),
    ("BinkShouldSkip", should_skip),
];

type Call = extern "system" fn(usize, usize, usize, usize) -> usize;

/// The library's own functions, by their place in [`WATCHED`].
static REAL: [AtomicUsize; 7] = [const { AtomicUsize::new(0) }; 7];

/// The address the title's import of `symbol` from `dll` is bound to:
/// `address` (the library's export), or the counter in front of it for a
/// watched function of the video library.
pub(crate) fn watched(dll: &str, symbol: &ImportSymbol, address: usize) -> usize {
    let ImportSymbol::Name { name, .. } = symbol else {
        return address;
    };
    if !dll.get(..LIBRARY.len()).is_some_and(|start| start.eq_ignore_ascii_case(LIBRARY)) {
        return address;
    }
    match WATCHED.iter().position(|(watched, _)| watched == name) {
        Some(i) => {
            REAL[i].store(address, Relaxed);
            WATCHED[i].1 as *const () as usize
        }
        None => address,
    }
}

fn real(i: usize, a: usize, b: usize, c: usize, d: usize) -> usize {
    // SAFETY: stored by `watched` before the title could call the counter;
    // the library's own function, called with the title's own arguments.
    let f: Call = unsafe { std::mem::transmute(REAL[i].load(Relaxed)) };
    f(a, b, c, d)
}

#[derive(Default)]
struct Players {
    /// Players open now, and the most at once since the last line.
    open: Vec<usize>,
    most: usize,
    opened: u64,
    /// Since the last line: frames shown, frames the library asked to skip,
    /// frames decoded with the time they took (sum and longest, ms).
    frames: u64,
    skipped: u64,
    decoded: u64,
    decoding: [f64; 2],
    /// Decodes begun and not yet finished, by player.
    begun: Vec<(usize, Instant)>,
    said: Option<Instant>,
}

static PLAYERS: Mutex<Players> = Mutex::new(Players {
    open: Vec::new(),
    most: 0,
    opened: 0,
    frames: 0,
    skipped: 0,
    decoded: 0,
    decoding: [0.0; 2],
    begun: Vec::new(),
    said: None,
});

fn players() -> std::sync::MutexGuard<'static, Players> {
    PLAYERS.lock().unwrap_or_else(|e| e.into_inner())
}

impl Players {
    fn finished(&mut self, player: usize) {
        if let Some(i) = self.begun.iter().position(|(p, _)| *p == player) {
            let ms = self.begun.swap_remove(i).1.elapsed().as_secs_f64() * 1e3;
            self.decoded += 1;
            self.decoding[0] += ms;
            self.decoding[1] = self.decoding[1].max(ms);
        }
    }

    /// The line, every two seconds of playback: `None` between lines.
    fn line(&mut self, every: Duration) -> Option<String> {
        let now = Instant::now();
        let since = now.duration_since(*self.said.get_or_insert(now));
        if since < every {
            return None;
        }
        let secs = since.as_secs_f64().max(1e-3);
        let line = format!(
            "video: {} players open ({} at once at most since the last line, {} opened since the start); {} frames \
             shown in {:.1} s ({:.1} a second), {} of them the library asked to skip for lateness; {} frames decoded, \
             {:.1} ms each on average, {:.1} ms at most",
            self.open.len(),
            self.most,
            self.opened,
            self.frames,
            secs,
            self.frames as f64 / secs,
            self.skipped,
            self.decoded,
            if self.decoded > 0 { self.decoding[0] / self.decoded as f64 } else { 0.0 },
            self.decoding[1]
        );
        self.said = Some(now);
        self.most = self.open.len();
        (self.frames, self.skipped, self.decoded, self.decoding) = (0, 0, 0, [0.0; 2]);
        Some(line)
    }
}

extern "system" fn open(a: usize, b: usize, c: usize, d: usize) -> usize {
    let player = real(OPEN, a, b, c, d);
    if player != 0 {
        let mut p = players();
        p.open.push(player);
        p.most = p.most.max(p.open.len());
        p.opened += 1;
        tracing::info!(open = p.open.len(), opened = p.opened, "video: a player opened");
    }
    player
}

extern "system" fn close(a: usize, b: usize, c: usize, d: usize) -> usize {
    {
        let mut p = players();
        p.open.retain(|&player| player != a);
        p.begun.retain(|(player, _)| *player != a);
        tracing::info!(open = p.open.len(), "video: a player closed");
    }
    real(CLOSE, a, b, c, d)
}

extern "system" fn decode_async(a: usize, b: usize, c: usize, d: usize) -> usize {
    {
        let mut p = players();
        p.begun.retain(|(player, _)| *player != a);
        p.begun.push((a, Instant::now()));
    }
    real(DECODE_ASYNC, a, b, c, d)
}

extern "system" fn decode_wait(a: usize, b: usize, c: usize, d: usize) -> usize {
    let done = real(DECODE_WAIT, a, b, c, d);
    if done != 0 {
        players().finished(a);
    }
    done
}

extern "system" fn decode(a: usize, b: usize, c: usize, d: usize) -> usize {
    players().begun.push((a, Instant::now()));
    let r = real(DECODE, a, b, c, d);
    players().finished(a);
    r
}

extern "system" fn next_frame(a: usize, b: usize, c: usize, d: usize) -> usize {
    let line = {
        let mut p = players();
        p.frames += 1;
        p.line(Duration::from_secs(2))
    };
    if let Some(line) = line {
        tracing::info!("{line}");
    }
    real(NEXT_FRAME, a, b, c, d)
}

extern "system" fn should_skip(a: usize, b: usize, c: usize, d: usize) -> usize {
    let skip = real(SHOULD_SKIP, a, b, c, d);
    // (The library answers with a 32-bit number.)
    if skip as u32 != 0 {
        players().skipped += 1;
    }
    skip
}

#[cfg(test)]
mod tests {
    use super::*;

    extern "system" fn library_open(_: usize, _: usize, _: usize, d: usize) -> usize {
        // The fourth argument stands for the player the library hands out.
        d
    }
    extern "system" fn library_none(_: usize, _: usize, _: usize, _: usize) -> usize {
        0
    }
    extern "system" fn library_one(_: usize, _: usize, _: usize, _: usize) -> usize {
        1
    }
    extern "system" fn library_sum(a: usize, b: usize, c: usize, d: usize) -> usize {
        a + b + c + d
    }

    fn name(name: &str) -> ImportSymbol {
        ImportSymbol::Name {
            hint: 0,
            name: name.to_owned(),
        }
    }

    /// Only the watched functions of the video library go through the
    /// counter; each call reaches the library with its arguments, and the
    /// players, frames, skips and decodes are counted.
    #[test]
    fn the_titles_video_calls_are_counted_on_their_way() {
        let own = |f: Call| f as *const () as usize;
        // Not the library, not a watched function, an ordinal: left alone.
        assert_eq!(watched("kernelx.dll", &name("BinkClose"), 7), 7);
        assert_eq!(watched("Bink2Scarlett.dll", &name("BinkSetVolume"), 7), 7);
        assert_eq!(watched("Bink2Scarlett.dll", &ImportSymbol::Ordinal(3), 7), 7);
        // The watched ones: the counter, in front of the library's own.
        let bound = |symbol: &str, library: Call| watched("bink2scarlett.DLL", &name(symbol), own(library));
        assert_eq!(bound("BinkOpenWithOptions", library_open), own(open));
        assert_eq!(bound("BinkClose", library_sum), own(close));
        assert_eq!(bound("BinkDoFrameAsyncMulti", library_one), own(decode_async));
        assert_eq!(bound("BinkDoFrameAsyncWait", library_one), own(decode_wait));
        assert_eq!(bound("BinkDoFrame", library_none), own(decode));
        assert_eq!(bound("BinkNextFrame", library_sum), own(next_frame));
        assert_eq!(bound("BinkShouldSkip", library_one), own(should_skip));

        // Two films at once, then one.
        assert_eq!(open(1, 2, 3, 0x1000), 0x1000);
        assert_eq!(open(1, 2, 3, 0x2000), 0x2000);
        assert_eq!(open(1, 2, 3, 0), 0, "a film that did not open is no player");
        {
            // (One lock at a time: the lock is not one a thread can take twice.)
            let p = players();
            assert_eq!((p.open.len(), p.most, p.opened), (2, 2, 2));
        }
        // The arguments reach the library as they were given.
        assert_eq!(close(0x2000, 1, 2, 3), 0x2006);
        assert_eq!(players().open.clone(), [0x1000]);

        // A frame: decoded on the library's threads, waited for, shown.
        assert_eq!(decode_async(0x1000, 0, 0, 0), 1);
        assert_eq!(decode_wait(0x1000, 0, 0, 0), 1);
        assert_eq!(decode(0x1000, 0, 0, 0), 0);
        assert_eq!(should_skip(0x1000, 0, 0, 0), 1);
        assert_eq!(next_frame(0x1000, 1, 1, 1), 0x1003);
        let mut p = players();
        assert_eq!((p.frames, p.skipped, p.decoded), (1, 1, 2));
        assert!(p.begun.is_empty());
        // No line before its time; then one, and the counts start again.
        assert_eq!(p.line(Duration::from_secs(3600)), None);
        let line = p.line(Duration::ZERO).expect("a line");
        assert!(line.starts_with("video: 1 players open (2 at once at most since the last line, 2 opened"), "{line}");
        assert!(line.contains("1 frames shown") && line.contains("2 frames decoded"), "{line}");
        assert_eq!((p.frames, p.skipped, p.decoded, p.most), (0, 0, 0, 1));
    }
}
