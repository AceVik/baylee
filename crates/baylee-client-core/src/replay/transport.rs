//! The transport (§6): where a replay stands, whether it plays, how fast,
//! and when the next step is due.
//!
//! One step forward is [`Out::Show`]: its frames are shown as a live table
//! shows them, with glides and cues. Every other move is [`Out::Jump`]: the
//! table is placed at the step, the log told again from its first line.
//! Pure: the clock comes in as [`Command::Tick`].

use std::time::Duration;

use super::timeline::Timeline;

/// What a replay is asked to do.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Command {
    /// Play if paused, pause if playing.
    Toggle,
    /// Play.
    Play,
    /// Pause.
    Pause,
    /// That many steps on (negative: back), and pause.
    Step(i8),
    /// That many turns on (negative: back; mid-turn, back is this turn's
    /// start), and pause.
    Turn(i8),
    /// To the next phase's first step.
    NextPhase,
    /// To the previous phase's first step (mid-phase: this phase's).
    PrevPhase,
    /// To step `n`.
    Seek(usize),
    /// To the first step.
    Start,
    /// To the last step.
    End,
    /// One speed up.
    Faster,
    /// One speed down.
    Slower,
    /// Time passed (the virtual clock's, so a paused clock holds a replay).
    Tick(Duration),
}

/// What the host shows for a command.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Out {
    /// Step `n` follows the one shown: show its frames as live.
    Show(usize),
    /// Place the table at step `n`.
    Jump(usize),
}

/// How fast a replay plays.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, PartialOrd, Ord)]
pub enum Speed {
    /// 0.5×.
    Half,
    /// 1×.
    #[default]
    One,
    /// 2×.
    Two,
    /// 4×.
    Four,
    /// 8×.
    Eight,
}

impl Speed {
    /// Every speed, slowest first.
    pub const ALL: [Self; 5] = [Self::Half, Self::One, Self::Two, Self::Four, Self::Eight];

    /// The factor time is divided by.
    #[must_use]
    pub const fn factor(self) -> f32 {
        match self {
            Self::Half => 0.5,
            Self::One => 1.0,
            Self::Two => 2.0,
            Self::Four => 4.0,
            Self::Eight => 8.0,
        }
    }

    /// How the pill writes it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Half => "0.5×",
            Self::One => "1×",
            Self::Two => "2×",
            Self::Four => "4×",
            Self::Eight => "8×",
        }
    }

    /// Whether the table sounds its cues at this speed (§7.3: none at 4×
    /// and 8×, where they would run into each other).
    #[must_use]
    pub const fn cues(self) -> bool {
        matches!(self, Self::Half | Self::One | Self::Two)
    }

    /// One faster, the fastest staying.
    #[must_use]
    pub fn faster(self) -> Self {
        let at = Self::ALL.iter().position(|s| *s == self).unwrap_or(1);
        Self::ALL[(at + 1).min(Self::ALL.len() - 1)]
    }

    /// One slower, the slowest staying.
    #[must_use]
    pub fn slower(self) -> Self {
        let at = Self::ALL.iter().position(|s| *s == self).unwrap_or(1);
        Self::ALL[at.saturating_sub(1)]
    }
}

/// How long step `i` stays on screen at 1× (§6): a step with log lines
/// 0.9 s, a quarter second more for every line past the first, at most 2 s;
/// without lines 0.3 s; a turn's first step 0.6 s more.
#[must_use]
pub fn dwell(timeline: &Timeline, i: usize) -> Duration {
    let lines = timeline.step(i).map_or(0, |m| m.lines);
    let base = if lines == 0 {
        0.3
    } else {
        (0.9 + 0.25 * (lines - 1) as f32).min(2.0)
    };
    let turn = if timeline.starts_turn(i) { 0.6 } else { 0.0 };
    Duration::from_secs_f32(base + turn)
}

/// Where a replay stands and how it moves.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Transport {
    /// The step shown.
    pub at: usize,
    /// Whether it plays on by itself.
    pub playing: bool,
    /// How fast.
    pub speed: Speed,
    /// How long the step shown has been on screen, at 1×.
    shown_for: Duration,
}

impl Transport {
    /// A transport at step `at`, paused.
    #[must_use]
    pub fn at(at: usize) -> Self {
        Self {
            at,
            ..Self::default()
        }
    }

    /// Carries out `command` over `timeline`; what to show, if anything
    /// moved.
    pub fn apply(&mut self, command: Command, timeline: &Timeline) -> Option<Out> {
        if timeline.is_empty() {
            self.playing = false;
            return None;
        }
        let last = timeline.last();
        match command {
            Command::Toggle if self.playing => self.pause(),
            Command::Toggle | Command::Play => {
                self.playing = true;
                self.shown_for = Duration::ZERO;
                // Played at the end: from the start again.
                if self.at >= last {
                    return self.jump(0);
                }
                None
            }
            Command::Pause => self.pause(),
            Command::Step(by) => {
                self.playing = false;
                let to = self.at.saturating_add_signed(isize::from(by)).min(last);
                if by == 1 && to == self.at + 1 {
                    self.at = to;
                    self.shown_for = Duration::ZERO;
                    Some(Out::Show(to))
                } else {
                    self.jump(to)
                }
            }
            Command::Turn(by) => {
                self.playing = false;
                let mut to = self.at;
                for _ in 0..by.unsigned_abs() {
                    to = if by > 0 {
                        timeline.next_turn(to).unwrap_or(last)
                    } else {
                        timeline.prev_turn(to).unwrap_or(0)
                    };
                }
                self.jump(to)
            }
            Command::NextPhase => {
                let to = timeline.next_phase(self.at).unwrap_or(last);
                self.jump(to)
            }
            Command::PrevPhase => {
                let to = timeline.prev_phase(self.at).unwrap_or(0);
                self.jump(to)
            }
            Command::Seek(to) => self.jump(to.min(last)),
            Command::Start => self.jump(0),
            Command::End => {
                self.playing = false;
                self.jump(last)
            }
            Command::Faster => {
                self.speed = self.speed.faster();
                None
            }
            Command::Slower => {
                self.speed = self.speed.slower();
                None
            }
            Command::Tick(dt) => self.tick(dt, timeline),
        }
    }

    fn pause(&mut self) -> Option<Out> {
        self.playing = false;
        None
    }

    /// Places the table at `to`; a jump to where it stands is no move.
    fn jump(&mut self, to: usize) -> Option<Out> {
        self.shown_for = Duration::ZERO;
        if to == self.at {
            return None;
        }
        self.at = to;
        Some(Out::Jump(to))
    }

    fn tick(&mut self, dt: Duration, timeline: &Timeline) -> Option<Out> {
        if !self.playing {
            return None;
        }
        let last = timeline.last();
        if self.at >= last {
            // The last step pauses; the end sheet holds.
            self.playing = false;
            return None;
        }
        self.shown_for += dt.mul_f32(self.speed.factor());
        let due = dwell(timeline, self.at);
        if self.shown_for < due {
            return None;
        }
        // What is left over carries, but never more than a step's worth: a
        // long frame does not skip steps nobody saw.
        self.shown_for = self.shown_for.saturating_sub(due).min(due);
        self.at += 1;
        if self.at >= last {
            self.playing = false;
        }
        Some(Out::Show(self.at))
    }
}
