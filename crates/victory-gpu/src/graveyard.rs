//! (PROJECT ONE) Objects waiting to be dropped once nothing on the card can
//! still read them (`crate::host::retire`).
//!
//! An object the runtime lets go of may still be named by a list: one the
//! card is running, one the title has submitted and a queue holds back, or
//! one a thread is recording with a handle it was given a moment before.
//! Only the first is counted (the submissions started and finished). The
//! others are covered by time: an object waits here
//!
//! 1. for a number of frames and a number of milliseconds, both; then
//! 2. for every submission started by then to have finished.
//!
//! Frames alone were not enough. Two frames are 660 ms at three frames a
//! second and 230 ms at nine: the run of 26/11, 8:25 lost the card at nine
//! frames a second with sixteen submissions sent ahead, the page fault
//! inside freed objects (a texture, a list's scratch buffer, two copies of
//! title memory), where fourteen runs at three to five frames a second had
//! lost nothing. And step 2 took the submissions started when the object
//! was let go, not those started while it waited.
//!
//! Both lines are first in, first out: what is due is taken from their
//! fronts (75,000 objects a second went through here in that run; looking
//! at every waiting object at every landing does not scale to a second's
//! worth of them).

use std::collections::VecDeque;

/// The present, for [`Graveyard::due`]: the frame, milliseconds on the
/// clock [`Graveyard::push`] was given, and the submissions started and
/// finished so far.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Now {
    pub(crate) frame: u64,
    pub(crate) ms: u64,
    pub(crate) started: u64,
    pub(crate) finished: u64,
}

struct Waiting<T> {
    /// Submissions started when it was let go.
    at: u64,
    /// The frame and the millisecond it waits for.
    frame: u64,
    ms: u64,
    object: T,
}

pub(crate) struct Graveyard<T> {
    /// Let go, their frames or milliseconds not over.
    waiting: VecDeque<Waiting<T>>,
    /// Their time over: each with the submission it waits for.
    armed: VecDeque<(u64, T)>,
}

impl<T> Graveyard<T> {
    pub(crate) const fn new() -> Self {
        Graveyard {
            waiting: VecDeque::new(),
            armed: VecDeque::new(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.waiting.is_empty() && self.armed.is_empty()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.waiting.len() + self.armed.len()
    }

    /// `object` was let go with `at` submissions started: it waits for
    /// `frame` and for `ms`.
    pub(crate) fn push(&mut self, at: u64, frame: u64, ms: u64, object: T) {
        self.waiting.push_back(Waiting { at, frame, ms, object });
    }

    /// Moves into `out` what can be dropped `now`. `again`: an object whose
    /// time is over waits for the submissions started by now, not only for
    /// those started when it was let go.
    ///
    /// An object behind one that waits longer waits with it (the lines are
    /// not sorted): later than it could go, never earlier.
    pub(crate) fn due(&mut self, now: Now, again: bool, out: &mut Vec<T>) {
        while self.waiting.front().is_some_and(|w| w.frame <= now.frame && w.ms <= now.ms) {
            let Some(w) = self.waiting.pop_front() else { break };
            let at = if again { w.at.max(now.started) } else { w.at };
            self.armed.push_back((at, w.object));
        }
        while self.armed.front().is_some_and(|(at, _)| *at <= now.finished) {
            let Some((_, object)) = self.armed.pop_front() else { break };
            out.push(object);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now(frame: u64, ms: u64, started: u64, finished: u64) -> Now {
        Now { frame, ms, started, finished }
    }

    fn taken(yard: &mut Graveyard<u32>, at: Now, again: bool) -> Vec<u32> {
        let mut out = Vec::new();
        yard.due(at, again, &mut out);
        out
    }

    /// An object waits for its frames and for its milliseconds, both: at
    /// nine frames a second two frames are over long before a second is.
    #[test]
    fn an_object_waits_for_its_frames_and_its_milliseconds() {
        let mut yard = Graveyard::new();
        // Let go at frame 10, 5,000 ms, nothing running: due at frame 12 and 6,000 ms.
        yard.push(7, 12, 6_000, 1);
        assert!(taken(&mut yard, now(12, 5_230, 7, 7), true).is_empty(), "two frames, 230 ms");
        assert!(taken(&mut yard, now(11, 6_500, 7, 7), true).is_empty(), "a second and a half, one frame");
        assert_eq!(taken(&mut yard, now(12, 6_000, 7, 7), true), [1]);
        assert!(yard.is_empty());
    }

    /// Its time over, it waits for the submissions started meanwhile: a
    /// list recorded with it while it waited is among them.
    #[test]
    fn its_time_over_it_waits_for_the_submissions_started_meanwhile() {
        let mut yard = Graveyard::new();
        yard.push(7, 12, 6_000, 1);
        // Sixteen submissions were sent ahead while it waited; 20 have landed.
        assert!(taken(&mut yard, now(12, 6_000, 23, 20), true).is_empty());
        assert!(taken(&mut yard, now(13, 6_100, 30, 22), true).is_empty(), "23 has not landed");
        assert_eq!(taken(&mut yard, now(13, 6_200, 31, 23), true), [1]);
        // As before this module (`again` false): the submissions started
        // when it was let go are enough.
        yard.push(7, 12, 6_000, 2);
        assert_eq!(taken(&mut yard, now(12, 6_000, 23, 7), false), [2]);
    }

    /// With no frames and no milliseconds to wait it goes as it did at
    /// first: when the submissions started as it was let go have finished.
    #[test]
    fn without_a_wait_it_goes_when_the_submissions_of_then_have_finished() {
        let mut yard = Graveyard::new();
        yard.push(9, 4, 1_000, 1);
        assert!(taken(&mut yard, now(4, 1_000, 12, 8), false).is_empty());
        assert_eq!(taken(&mut yard, now(4, 1_000, 12, 9), false), [1]);
    }

    /// First in, first out, and nothing is lost or dropped twice.
    #[test]
    fn everything_let_go_is_dropped_once_in_order() {
        let mut yard = Graveyard::new();
        for i in 0..1_000u32 {
            yard.push(u64::from(i / 10), u64::from(i / 100) + 2, u64::from(i) + 1_000, i);
        }
        assert_eq!(yard.len(), 1_000);
        let mut all = Vec::new();
        for step in 0..=120u64 {
            // Frames, the clock and the submissions all go on.
            yard.due(now(step, step * 20, step, step.saturating_sub(1)), true, &mut all);
        }
        assert_eq!(all, (0..1_000).collect::<Vec<_>>());
        assert!(yard.is_empty());
    }
}
