//! A replay's timeline (§6): one step per distinct view, the turns and
//! phases they fall in, and the bands where a check is weaker than exact.
//!
//! Built once per tape ([`Timeline::of`]); everything the transport bar
//! draws and every jump the transport makes is a lookup here.

use baylee_core::ids::PlayerId;

use super::tape::{Place, Tape};
use super::wire::{Check, StopReason};

/// What the timeline keeps of one step.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Mark {
    /// Where the game stands.
    pub place: Place,
    /// How far the step is known to be the recorded game's.
    pub check: Check,
    /// The log lines this step adds.
    pub lines: usize,
    /// The record input after which it was made.
    pub n: Option<u64>,
}

/// One turn: a cell of the scrubber, as wide as its steps.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Turn {
    /// The turn's number.
    pub turn: u32,
    /// Whose turn it is.
    pub active: PlayerId,
    /// Its first step.
    pub first: usize,
    /// How many steps it holds.
    pub len: usize,
}

impl Turn {
    /// One past its last step.
    #[must_use]
    pub const fn end(&self) -> usize {
        self.first + self.len
    }
}

/// A run of steps under one check weaker than exact.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Band {
    /// The first step.
    pub from: usize,
    /// One past the last.
    pub to: usize,
    /// `Seen` (a quiet underline) or `Unverified` (a hatch).
    pub check: Check,
}

/// A replay's steps, turns, phases and bands.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Timeline {
    steps: Vec<Mark>,
    turns: Vec<Turn>,
    phases: Vec<usize>,
    bands: Vec<Band>,
    /// How the tape ends, when it ends before the game does.
    stopped: Option<StopReason>,
}

impl Timeline {
    /// The timeline of `tape`.
    #[must_use]
    pub fn of(tape: &Tape) -> Self {
        let mut timeline = Self::default();
        let mut log_before = 0;
        for (i, frame) in tape.frames.iter().enumerate() {
            let place = frame.place;
            timeline.steps.push(Mark {
                place,
                check: frame.check,
                lines: frame.log_len.saturating_sub(log_before),
                n: frame.n,
            });
            log_before = frame.log_len;
            match timeline.turns.last_mut() {
                Some(turn) if turn.turn == place.turn => turn.len += 1,
                _ => timeline.turns.push(Turn {
                    turn: place.turn,
                    active: place.active,
                    first: i,
                    len: 1,
                }),
            }
            let new_phase = i == 0
                || tape.frames[i - 1].place.turn != place.turn
                || tape.frames[i - 1].place.phase != place.phase;
            if new_phase {
                timeline.phases.push(i);
            }
            if frame.check != Check::Exact {
                match timeline.bands.last_mut() {
                    Some(band) if band.to == i && band.check == frame.check => band.to += 1,
                    _ => timeline.bands.push(Band {
                        from: i,
                        to: i + 1,
                        check: frame.check,
                    }),
                }
            }
        }
        timeline.stopped = match tape.stop.stop {
            StopReason::End => None,
            other => Some(other),
        };
        timeline
    }

    /// How many steps.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// The last step's index (0 for an empty timeline).
    #[must_use]
    pub fn last(&self) -> usize {
        self.steps.len().saturating_sub(1)
    }

    /// Step `i`.
    #[must_use]
    pub fn step(&self, i: usize) -> Option<&Mark> {
        self.steps.get(i)
    }

    /// The turns, in order.
    #[must_use]
    pub fn turns(&self) -> &[Turn] {
        &self.turns
    }

    /// Every phase's first step, in order (each turn's first is one).
    #[must_use]
    pub fn phases(&self) -> &[usize] {
        &self.phases
    }

    /// The runs weaker than exact.
    #[must_use]
    pub fn bands(&self) -> &[Band] {
        &self.bands
    }

    /// Why the tape ends before the game does (`None`: it reaches the end).
    #[must_use]
    pub const fn stopped(&self) -> Option<StopReason> {
        self.stopped
    }

    /// The turn step `i` falls in.
    #[must_use]
    pub fn turn_of(&self, i: usize) -> Option<&Turn> {
        let at = self
            .turns
            .partition_point(|t| t.first <= i)
            .checked_sub(1)?;
        self.turns.get(at)
    }

    /// Whether step `i` is its turn's first.
    #[must_use]
    pub fn starts_turn(&self, i: usize) -> bool {
        self.turn_of(i).is_some_and(|t| t.first == i)
    }

    /// The next turn's first step, if there is a next turn.
    #[must_use]
    pub fn next_turn(&self, i: usize) -> Option<usize> {
        self.turns.iter().find(|t| t.first > i).map(|t| t.first)
    }

    /// Mid-turn: this turn's first step. At a turn's first: the previous
    /// turn's. At the very first step: none.
    #[must_use]
    pub fn prev_turn(&self, i: usize) -> Option<usize> {
        self.turns
            .iter()
            .rev()
            .find(|t| t.first < i)
            .map(|t| t.first)
    }

    /// The next phase's first step.
    #[must_use]
    pub fn next_phase(&self, i: usize) -> Option<usize> {
        self.phases.iter().copied().find(|&p| p > i)
    }

    /// Mid-phase: this phase's first step; at a phase's first, the previous
    /// phase's.
    #[must_use]
    pub fn prev_phase(&self, i: usize) -> Option<usize> {
        self.phases.iter().rev().copied().find(|&p| p < i)
    }

    /// The step under a point `fraction` of the way along the scrubber,
    /// where every step is as wide as every other.
    #[must_use]
    pub fn step_at(&self, fraction: f32) -> usize {
        if self.steps.is_empty() {
            return 0;
        }
        let at = (fraction.clamp(0.0, 1.0) * self.steps.len() as f32).floor() as usize;
        at.min(self.last())
    }

    /// Where step `i`'s middle lies along the scrubber, `0..=1`.
    #[must_use]
    pub fn fraction_of(&self, i: usize) -> f32 {
        if self.steps.is_empty() {
            return 0.0;
        }
        (i.min(self.last()) as f32 + 0.5) / self.steps.len() as f32
    }
}
