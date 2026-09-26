//! Original 6/8 theme, orchestrated as one continuous performance.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)] // bounded MIDI/tick/sample conversions
use super::{RATE, orchestra::Orchestra};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

/// Musical destination; changes are admitted on the next bar line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum Mood {
    /// Moonlit garden, harp and woodwinds over strings.
    #[default]
    Sanctuary,
    /// Horns, bowed ostinato and orchestral percussion.
    Battle,
    /// A four-bar rising cadence, ending in D major.
    Victory,
    /// A quiet descending minor cadence.
    Defeat,
    /// An open suspended cadence, resolving without a triumph.
    Draw,
}
impl Mood {
    fn of(value: u32) -> Self {
        match value {
            1 => Self::Battle,
            2 => Self::Victory,
            3 => Self::Defeat,
            4 => Self::Draw,
            _ => Self::Sanctuary,
        }
    }
    fn ending(self) -> bool {
        matches!(self, Self::Victory | Self::Defeat | Self::Draw)
    }
}

/// Lock-free message from the game to the musical conductor. A packed word
/// makes mood and energy an atomic snapshot, also on wasm without threads.
#[derive(Debug, Default)]
pub struct ScoreControl(AtomicU32);
impl ScoreControl {
    /// Request a mood and an intensity in 0..=1. Invalid intensity is silence
    /// of the action layer, not a NaN in the audio graph.
    pub fn set(&self, mood: Mood, intensity: f32) {
        let energy = if intensity.is_finite() {
            intensity.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.0.store(
            u32::from(mood as u8) | ((energy * 1000.0).round() as u32) << 8,
            Ordering::Relaxed,
        );
    }
    fn get(&self) -> (Mood, f32) {
        let word = self.0.load(Ordering::Relaxed);
        (Mood::of(word & 255), (word >> 8) as f32 / 1000.0)
    }
}

/// Endless stereo sampler. Its transport, voices and reverb outlive every
/// game state and mute toggle. Only newly scheduled notes change orchestration.
pub struct Tune {
    control: Arc<ScoreControl>,
    orchestra: Orchestra,
    mood: Mood,
    ending_bars: u8,
    bar: u64,
    tick: u8,
    until_tick: f64,
    bpm: f64,
    energy: f32,
    right: Option<f32>,
}
impl Default for Tune {
    fn default() -> Self {
        Self::new()
    }
}
impl Tune {
    /// Start the sanctuary score with a private conductor.
    #[must_use]
    pub fn new() -> Self {
        Self::with_control(Arc::new(ScoreControl::default()))
    }
    /// Start a performance whose future bars follow this conductor.
    #[must_use]
    pub fn with_control(control: Arc<ScoreControl>) -> Self {
        Self {
            control,
            orchestra: Orchestra::default(),
            mood: Mood::Sanctuary,
            ending_bars: 0,
            bar: 0,
            tick: 0,
            until_tick: 0.0,
            bpm: 60.0,
            energy: 0.0,
            right: None,
        }
    }
    /// Render the next stereo frame, without allocation or I/O.
    pub fn frame(&mut self) -> [f32; 2] {
        if self.until_tick <= 0.0 {
            if self.tick == 0 {
                self.conduct();
            }
            self.schedule();
            self.tick += 1;
            if self.tick == 12 {
                self.tick = 0;
                self.bar += 1;
                self.ending_bars = self.ending_bars.saturating_sub(1);
            }
            self.until_tick += f64::from(RATE) * 60.0 / (self.bpm * 6.0);
        }
        let target = match self.mood {
            Mood::Battle => 72.0 + f64::from(self.energy) * 28.0,
            Mood::Victory => 68.0,
            Mood::Defeat => 54.0,
            Mood::Draw => 57.0,
            Mood::Sanctuary => 60.0,
        };
        // Eight-second tempo inertia, per sample: no pitch change, beat jump or restart.
        self.bpm += (target - self.bpm) / (f64::from(RATE) * 8.0);
        self.until_tick -= 1.0;
        self.orchestra.frame()
    }
    fn conduct(&mut self) {
        let (wanted, energy) = self.control.get();
        // Finish an outcome cadence even when its sheet was dismissed at once.
        if self.ending_bars == 0 && wanted != self.mood {
            self.mood = wanted;
            if wanted.ending() {
                self.ending_bars = 4;
            }
        }
        let wanted = if self.mood == Mood::Battle {
            energy
        } else {
            0.0
        };
        self.energy += (wanted - self.energy).clamp(-0.18, 0.24);
    }
    fn chord(&self) -> (u8, u8) {
        if self.ending_bars > 0 {
            let i = usize::from(4 - self.ending_bars);
            return match self.mood {
                Mood::Victory => [(58, 4), (60, 4), (57, 4), (62, 4)][i],
                Mood::Defeat => [(62, 3), (55, 3), (57, 4), (62, 3)][i],
                _ => [(58, 4), (60, 4), (62, 5), (62, 3)][i],
            };
        }
        if self.mood.ending() {
            return (62, if self.mood == Mood::Victory { 4 } else { 3 });
        }
        CHORDS[(self.bar % 32) as usize]
    }
    fn play(&mut self, instrument: usize, pitch: u8, ticks: f32, gain: f32, pan: f32) {
        let seconds = ticks * 60.0 / (self.bpm as f32 * 6.0);
        // Small deterministic phrase shaping, not random detuning of recordings.
        let human = 0.94 + 0.06 * ((self.bar * 7 + u64::from(self.tick) * 3) % 5) as f32 / 4.0;
        self.orchestra
            .note(instrument, pitch, seconds, gain * human, pan);
    }
    #[allow(clippy::too_many_lines)] // one scored bar: foundation, melody, counterline and rhythmic response
    fn schedule(&mut self) {
        let (root, third) = self.chord();
        let t = self.tick;
        let phrase = (self.bar % 32) as usize;
        let battle = self.mood == Mood::Battle;
        let ending = self.mood.ending();
        let e = self.energy;
        let arc = 0.78 + 0.22 * (std::f32::consts::PI * (phrase % 8) as f32 / 8.0).sin();
        if t == 0 {
            self.play(2, root - 12, 11.8, 0.24 * arc, 0.22);
            self.play(1, root + third, 11.4, 0.12 * arc, -0.12);
            self.play(0, root + 7, 10.8, 0.13 * arc, -0.42);
            if battle || self.mood == Mood::Victory {
                self.play(
                    if e > 0.7 { 10 } else { 9 },
                    root,
                    8.5,
                    0.16 + e * 0.12,
                    0.25,
                );
            }
            if (battle && e > 0.25) || (self.ending_bars == 4 && self.mood == Mood::Victory) {
                self.play(12, root - 24, 4.0, 0.12 + e * 0.22, 0.0);
                if phrase.is_multiple_of(4) || ending {
                    self.play(15, 60, 10.0, 0.055 + e * 0.08, 0.3);
                }
            }
        }
        if t.is_multiple_of(2) && (!ending || self.ending_bars > 0) {
            let degrees = [0, 7, 12, third, 7, 12 + third];
            let pitch = root + degrees[usize::from(t / 2)];
            self.play(
                if pitch < 58 {
                    5
                } else if pitch < 70 {
                    6
                } else {
                    7
                },
                pitch,
                4.6,
                (if battle { 0.115 } else { 0.19 }) * arc,
                -0.27 + f32::from(t) * 0.04,
            );
        }
        if !ending {
            let (pitch, length) = MELODY[phrase][usize::from(t)];
            if pitch > 0 {
                let instrument = if battle {
                    9
                } else if (phrase / 8).is_multiple_of(2) {
                    8
                } else {
                    11
                };
                let pitch = if battle {
                    pitch.saturating_sub(12)
                } else {
                    pitch
                };
                self.play(
                    instrument,
                    pitch,
                    f32::from(length) * 0.92,
                    0.20 + e * 0.12,
                    if battle { 0.15 } else { -0.12 },
                );
            }
            // Answering line rests between phrases so the theme has room to breathe.
            if (phrase % 8 >= 4) && (t == 3 || t == 9) {
                self.play(
                    if battle { 0 } else { 2 },
                    root + if t == 3 { third } else { 7 },
                    3.4,
                    0.11 * arc,
                    0.32,
                );
            }
        } else if self.ending_bars > 0 && (t == 0 || t == 6) {
            let rise = 4 - self.ending_bars;
            let pitch = match self.mood {
                Mood::Victory => root + if t == 0 { 7 } else { 12 + third },
                Mood::Defeat => root + if t == 0 { 7 } else { third },
                _ => root + if rise < 2 { 7 } else { 12 },
            };
            self.play(
                if self.mood == Mood::Victory { 9 } else { 11 },
                pitch,
                5.5,
                0.23,
                -0.08,
            );
        } else if ending && self.bar.is_multiple_of(4) && t == 0 {
            self.play(6, root + 7, 10.0, 0.16, -0.2);
        }
        if battle && e > 0.18 && (t.is_multiple_of(2) || e > 0.78) {
            let pitch = root + [0, 7, 12, 7, third, 7][usize::from(t) % 6];
            self.play(
                3 + ((self.bar + u64::from(t)) % 2) as usize,
                pitch,
                0.75,
                (0.07 + 0.18 * e) * if t.is_multiple_of(6) { 1.2 } else { 0.8 },
                -0.45,
            );
        }
        if battle && e > 0.4 && (t == 5 || t == 11 || (e > 0.8 && t == 10)) {
            self.play(
                13 + (self.bar % 2) as usize,
                60,
                0.8,
                0.045 + e * 0.11,
                0.12,
            );
        }
        if battle && e > 0.6 && t == 6 {
            self.play(12, root - 17, 3.0, 0.18 * e, 0.0);
        }
    }
}
impl Iterator for Tune {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if let Some(right) = self.right.take() {
            return Some(right);
        }
        let [left, right] = self.frame();
        self.right = Some(right);
        Some(left)
    }
}

const CHORDS: [(u8, u8); 32] = [
    (62, 3),
    (58, 4),
    (60, 4),
    (57, 4),
    (62, 3),
    (58, 4),
    (65, 4),
    (60, 4),
    (62, 3),
    (55, 4),
    (58, 4),
    (57, 4),
    (62, 3),
    (58, 4),
    (65, 4),
    (60, 4),
    (55, 3),
    (58, 4),
    (57, 4),
    (62, 3),
    (65, 4),
    (60, 4),
    (62, 3),
    (58, 4),
    (65, 4),
    (60, 4),
    (55, 3),
    (57, 4),
    (58, 4),
    (60, 4),
    (57, 4),
    (57, 4),
];
const MELODY: [[(u8, u8); 12]; 32] = [
    [
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        (69, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (74, 2),
        (0, 0),
        (76, 2),
        (0, 0),
        (77, 2),
        (0, 0),
    ],
    [
        (79, 4),
        (0, 0),
        (0, 0),
        (0, 0),
        (77, 2),
        (0, 0),
        (74, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        (72, 2),
        (0, 0),
        (74, 2),
        (0, 0),
        (72, 2),
        (0, 0),
        (69, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        (67, 2),
        (0, 0),
        (69, 2),
        (0, 0),
        (72, 2),
        (0, 0),
        (76, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        (77, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (76, 2),
        (0, 0),
        (74, 2),
        (0, 0),
        (76, 2),
        (0, 0),
    ],
    [
        (74, 4),
        (0, 0),
        (0, 0),
        (0, 0),
        (71, 2),
        (0, 0),
        (67, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        (70, 2),
        (0, 0),
        (74, 2),
        (0, 0),
        (77, 2),
        (0, 0),
        (79, 4),
        (0, 0),
        (0, 0),
        (0, 0),
        (77, 2),
        (0, 0),
    ],
    [
        (76, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (73, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        (74, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (81, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        (82, 4),
        (0, 0),
        (0, 0),
        (0, 0),
        (81, 2),
        (0, 0),
        (77, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        (81, 2),
        (0, 0),
        (79, 2),
        (0, 0),
        (77, 2),
        (0, 0),
        (72, 4),
        (0, 0),
        (0, 0),
        (0, 0),
        (77, 2),
        (0, 0),
    ],
    [
        (76, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (84, 4),
        (0, 0),
        (0, 0),
        (0, 0),
        (81, 2),
        (0, 0),
    ],
    [
        (79, 4),
        (0, 0),
        (0, 0),
        (0, 0),
        (82, 2),
        (0, 0),
        (81, 2),
        (0, 0),
        (79, 2),
        (0, 0),
        (77, 2),
        (0, 0),
    ],
    [
        (77, 4),
        (0, 0),
        (0, 0),
        (0, 0),
        (74, 2),
        (0, 0),
        (70, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        (73, 2),
        (0, 0),
        (76, 2),
        (0, 0),
        (81, 2),
        (0, 0),
        (79, 4),
        (0, 0),
        (0, 0),
        (0, 0),
        (76, 2),
        (0, 0),
    ],
    [
        (74, 12),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        (81, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (84, 4),
        (0, 0),
        (0, 0),
        (0, 0),
        (81, 2),
        (0, 0),
    ],
    [
        (79, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (76, 4),
        (0, 0),
        (0, 0),
        (0, 0),
        (72, 2),
        (0, 0),
    ],
    [
        (74, 2),
        (0, 0),
        (76, 2),
        (0, 0),
        (77, 2),
        (0, 0),
        (81, 2),
        (0, 0),
        (79, 2),
        (0, 0),
        (77, 2),
        (0, 0),
    ],
    [
        (74, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (77, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        (81, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (84, 4),
        (0, 0),
        (0, 0),
        (0, 0),
        (86, 2),
        (0, 0),
    ],
    [
        (84, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (79, 4),
        (0, 0),
        (0, 0),
        (0, 0),
        (76, 2),
        (0, 0),
    ],
    [
        (77, 2),
        (0, 0),
        (79, 2),
        (0, 0),
        (82, 2),
        (0, 0),
        (86, 4),
        (0, 0),
        (0, 0),
        (0, 0),
        (82, 2),
        (0, 0),
    ],
    [
        (81, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (76, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        (77, 4),
        (0, 0),
        (0, 0),
        (0, 0),
        (74, 2),
        (0, 0),
        (70, 4),
        (0, 0),
        (0, 0),
        (0, 0),
        (74, 2),
        (0, 0),
    ],
    [
        (76, 4),
        (0, 0),
        (0, 0),
        (0, 0),
        (79, 2),
        (0, 0),
        (84, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        (81, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (76, 2),
        (0, 0),
        (73, 2),
        (0, 0),
        (76, 2),
        (0, 0),
    ],
    [
        (69, 6),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (64, 2),
        (0, 0),
        (65, 2),
        (0, 0),
        (67, 2),
        (0, 0),
    ],
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[allow(clippy::float_cmp)] // exact sample equality proves the request does not fade the current bar
    fn requests_wait_for_the_bar_and_never_reset_the_transport() {
        let control = Arc::new(ScoreControl::default());
        let mut tune = Tune::with_control(control.clone());
        let mut reference = Tune::new();
        for _ in 0..RATE / 2 {
            assert_eq!(tune.frame(), reference.frame());
        }
        control.set(Mood::Battle, 1.0);
        for _ in 0..RATE {
            assert_eq!(tune.frame(), reference.frame());
        }
        assert_eq!(tune.mood, Mood::Sanctuary);
        for _ in 0..RATE {
            tune.frame();
        }
        assert_eq!(tune.mood, Mood::Battle);
        assert!(tune.bar > 0);
        assert!(tune.bpm > 60.0 && tune.bpm < 70.0, "tempo slews");
    }
    #[test]
    fn a_dismissed_ending_finishes_its_cadence_before_returning() {
        let control = Arc::new(ScoreControl::default());
        let mut tune = Tune::with_control(control.clone());
        control.set(Mood::Victory, 0.0);
        tune.frame();
        assert_eq!(tune.ending_bars, 4);
        control.set(Mood::Sanctuary, 0.0);
        for _ in 0..RATE * 4 {
            tune.frame();
        }
        assert_eq!(tune.mood, Mood::Victory);
        for _ in 0..RATE * 5 {
            tune.frame();
        }
        assert_eq!(tune.mood, Mood::Sanctuary);
        assert!(tune.bar >= 4);
    }
    #[test]
    fn every_orchestration_is_finite_audible_and_leaves_headroom() {
        for mood in [
            Mood::Sanctuary,
            Mood::Battle,
            Mood::Victory,
            Mood::Defeat,
            Mood::Draw,
        ] {
            let control = Arc::new(ScoreControl::default());
            control.set(mood, 1.0);
            let mut tune = Tune::with_control(control);
            let (mut peak, mut power) = (0.0_f32, 0.0_f64);
            let mut previous = [0.0; 2];
            let mut jump = 0.0_f32;
            for _ in 0..RATE * 12 {
                let frame = tune.frame();
                for i in 0..2 {
                    assert!(frame[i].is_finite());
                    peak = peak.max(frame[i].abs());
                    power += f64::from(frame[i]).powi(2);
                    jump = jump.max((frame[i] - previous[i]).abs());
                }
                previous = frame;
            }
            let rms = (power / f64::from(RATE * 24)).sqrt();
            eprintln!("{mood:?}: peak {peak:.3}, rms {rms:.3}, jump {jump:.3}");
            assert!((0.015..0.80).contains(&peak));
            assert!(rms > 0.003 && rms < 0.22);
            assert!(jump < 0.20);
        }
    }
    #[test]
    fn invalid_energy_cannot_poison_the_audio_clock() {
        let control = ScoreControl::default();
        control.set(Mood::Battle, f32::NAN);
        assert_eq!(control.get(), (Mood::Battle, 0.0));
        control.set(Mood::Battle, 3.0);
        assert_eq!(control.get(), (Mood::Battle, 1.0));
    }
}
