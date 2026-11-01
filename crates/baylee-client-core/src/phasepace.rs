//! How fast the table shows a turn's steps (owner, 29.10.2026: "phase
//! transitions at the table should be smooth, good-looking animations, and
//! slower, so a person can follow what happened").
//!
//! The engine moves through the steps as fast as the seats pass, and when
//! the house or a standing order passes, three steps go by between two
//! frames: the seat bars' light jumped from the upkeep to the end of combat
//! and nobody saw the turn go round. The bars now draw the step this pace
//! shows rather than the view's. Every step the views passed through is
//! held for at least [`STEP_HOLD`] before the next is shown, so the light
//! walks the row a step at a time; only steps a view stood in are shown, so
//! it never passes through a step the game skipped (CR 508.8).
//!
//! It never stands in the way of an answer: when this seat is asked
//! something, the steps still waiting run by at [`CATCH_UP`] each, so the
//! light reaches the step asked about in a fraction of a second instead of
//! jumping there (in another seat's turn a seat is asked at most steps, and
//! a jump each time was the cut this was written to end; recorded
//! 30.10.2026). Reduced motion goes straight to where the game is. And it
//! never falls far behind: past [`MAX_BEHIND`] waiting steps the oldest are
//! dropped, so a long run of passes is shown as its last few.

use std::collections::VecDeque;

use baylee_core::ids::PlayerId;
use baylee_view::{Phase, Step};

use crate::automation::RailRow;

/// The least time one step is shown before the next, in seconds.
pub const STEP_HOLD: f32 = 0.6;

/// The least time one step is shown while this seat must answer.
pub const CATCH_UP: f32 = 0.15;

/// The most steps waiting to be shown; older ones are dropped.
pub const MAX_BEHIND: usize = 4;

/// One step of one turn, as the bars show it: the view's own words for it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Shown {
    /// The turn's number.
    pub turn: u32,
    /// Whose turn it is.
    pub active: PlayerId,
    /// The phase.
    pub phase: Phase,
    /// The step.
    pub step: Step,
}

impl Shown {
    /// The step as the bars' tiles name it.
    #[must_use]
    pub fn row(&self) -> RailRow {
        RailRow::current(self.phase, self.step)
    }
}

/// The step shown, and the steps waiting to be.
#[derive(Clone, Debug, Default)]
pub struct PhasePace {
    shown: Option<Shown>,
    waiting: VecDeque<Shown>,
    /// The clock when `shown` was last changed.
    at: f32,
}

impl PhasePace {
    /// What is shown, if anything has been.
    #[must_use]
    pub const fn shown(&self) -> Option<Shown> {
        self.shown
    }

    /// Takes the step the view stands in (`seen`), at `now` seconds, and
    /// moves on what is shown where it is time to. `asked` is whether this
    /// seat must answer something now, which holds each step only
    /// [`CATCH_UP`]; `still` is whether motion is reduced, which goes
    /// straight to `seen`. Whether what is shown changed.
    pub fn follow(&mut self, now: f32, seen: Shown, asked: bool, still: bool) -> bool {
        if self.waiting.back().copied().or(self.shown) != Some(seen) {
            self.waiting.push_back(seen);
        }
        while self.waiting.len() > MAX_BEHIND {
            self.waiting.pop_front();
        }
        if self.shown.is_none() || still {
            let Some(newest) = self.waiting.pop_back() else {
                return false;
            };
            self.waiting.clear();
            return self.show(now, newest);
        }
        let hold = if asked { CATCH_UP } else { STEP_HOLD };
        if now - self.at >= hold
            && let Some(next) = self.waiting.pop_front()
        {
            return self.show(now, next);
        }
        false
    }

    /// Whether steps are still waiting to be shown.
    #[must_use]
    pub fn behind(&self) -> bool {
        !self.waiting.is_empty()
    }

    /// Forgets everything: a new game, or a replay's jump.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn show(&mut self, now: f32, step: Shown) -> bool {
        let changed = self.shown != Some(step);
        self.shown = Some(step);
        self.at = now;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(row: RailRow) -> Shown {
        let (phase, step) = match row {
            RailRow::Untap => (Phase::Beginning, Step::Untap),
            RailRow::Upkeep => (Phase::Beginning, Step::Upkeep),
            RailRow::Draw => (Phase::Beginning, Step::Draw),
            RailRow::Main1 => (Phase::FirstMain, Step::Main),
            RailRow::CombatBegin => (Phase::Combat, Step::CombatBegin),
            RailRow::Attackers => (Phase::Combat, Step::DeclareAttackers),
            RailRow::Blockers => (Phase::Combat, Step::DeclareBlockers),
            RailRow::Damage => (Phase::Combat, Step::CombatDamage),
            RailRow::CombatEnd => (Phase::Combat, Step::CombatEnd),
            RailRow::Main2 => (Phase::SecondMain, Step::Main),
            RailRow::EndStep => (Phase::Ending, Step::End),
            RailRow::Cleanup => (Phase::Ending, Step::Cleanup),
        };
        let shown = Shown {
            turn: 3,
            active: PlayerId::new(1),
            phase,
            step,
        };
        assert_eq!(shown.row(), row, "the test's own table");
        shown
    }

    /// The house passes from the upkeep to the end of combat in one breath;
    /// the bars show each step a view stood in, one hold apart, and none
    /// the views did not.
    #[test]
    fn each_step_the_views_passed_is_held_before_the_next() {
        let mut pace = PhasePace::default();
        assert!(pace.follow(0.0, at(RailRow::Upkeep), false, false));
        for row in [RailRow::Draw, RailRow::Main1, RailRow::CombatBegin] {
            pace.follow(0.01, at(row), false, false);
        }
        assert_eq!(pace.shown(), Some(at(RailRow::Upkeep)), "nothing yet");
        let mut walked = Vec::new();
        let mut now = 0.0;
        while pace.behind() || walked.is_empty() {
            now += 0.05;
            if pace.follow(now, at(RailRow::CombatBegin), false, false) {
                walked.push((pace.shown().unwrap().row(), now));
            }
        }
        let rows: Vec<RailRow> = walked.iter().map(|(r, _)| *r).collect();
        assert_eq!(rows, [RailRow::Draw, RailRow::Main1, RailRow::CombatBegin]);
        for pair in walked.windows(2) {
            assert!(pair[1].1 - pair[0].1 >= STEP_HOLD - 1e-4, "{walked:?}");
        }
    }

    /// With motion reduced, the bars go straight to where the game is, and
    /// what waited is forgotten.
    #[test]
    fn reduced_motion_goes_straight_to_the_step() {
        let mut pace = PhasePace::default();
        pace.follow(0.0, at(RailRow::Upkeep), false, false);
        pace.follow(0.1, at(RailRow::Draw), false, false);
        assert!(pace.follow(0.2, at(RailRow::Main1), false, true));
        assert_eq!(pace.shown(), Some(at(RailRow::Main1)));
        assert!(!pace.behind());
    }

    /// Asked something, the steps waiting run by quickly instead of being
    /// skipped: each one is shown, and the step asked about is reached
    /// well within one ordinary hold per step.
    #[test]
    fn an_answer_runs_the_light_quickly_to_the_step() {
        let mut pace = PhasePace::default();
        pace.follow(0.0, at(RailRow::Upkeep), false, false);
        pace.follow(0.01, at(RailRow::Draw), false, false);
        pace.follow(0.01, at(RailRow::Main1), false, false);
        let mut walked = Vec::new();
        let mut now = 0.01;
        while pace.behind() {
            now += 0.01;
            if pace.follow(now, at(RailRow::Main1), true, false) {
                walked.push(pace.shown().unwrap().row());
            }
        }
        assert_eq!(walked, [RailRow::Draw, RailRow::Main1], "no step skipped");
        assert!(now < 2.0 * STEP_HOLD, "caught up only at {now} s");
        assert!(now >= 2.0 * CATCH_UP - 0.02, "it jumped: {now} s");
    }

    /// A long run of passes is shown as its last few steps.
    #[test]
    fn it_never_falls_more_than_a_few_steps_behind() {
        let mut pace = PhasePace::default();
        pace.follow(0.0, at(RailRow::Untap), false, false);
        let rows = [
            RailRow::Upkeep,
            RailRow::Draw,
            RailRow::Main1,
            RailRow::CombatBegin,
            RailRow::Attackers,
            RailRow::CombatEnd,
            RailRow::Main2,
        ];
        for row in rows {
            pace.follow(0.01, at(row), false, false);
        }
        let mut now = 0.01;
        let mut walked = Vec::new();
        while pace.behind() {
            now += STEP_HOLD;
            if pace.follow(now, at(RailRow::Main2), false, false) {
                walked.push(pace.shown().unwrap().row());
            }
        }
        assert_eq!(walked.len(), MAX_BEHIND);
        assert_eq!(walked.last(), Some(&RailRow::Main2));
    }
}
