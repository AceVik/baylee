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
    /// Moonlit garden, expressive piano and harp over divided strings.
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
    fn piano(&mut self, pitch: u8, ticks: f32, gain: f32, touch: f32) {
        let seconds = ticks * 60.0 / (self.bpm as f32 * 6.0);
        self.orchestra.piano(pitch, seconds, gain, touch);
    }

    fn schedule(&mut self) {
        let (root, third) = self.chord();
        let phrase = (self.bar % 32) as usize;
        let arc = 0.72 + 0.28 * (std::f32::consts::PI * (phrase % 8) as f32 / 8.0).sin();
        self.foundation(root, third, arc);
        self.keyboard(root, third, arc);
        self.theme(root, third, arc);
        self.percussion(root, third);
    }

    fn foundation(&mut self, root: u8, third: u8, arc: f32) {
        let t = self.tick;
        let phrase = (self.bar % 32) as usize;
        let e = self.energy;
        let battle = self.mood == Mood::Battle;
        // Inversions make a bass melody instead of jumping root to root.
        let bass = if self.mood.ending() {
            root - 12
        } else {
            BASSES[phrase]
        };
        if t == 0 {
            self.play(2, bass, 11.8, 0.20 * arc, 0.22);
            self.play(1, root + third, 11.4, 0.085 * arc, -0.12);
            self.play(0, root + 7, 10.8, 0.09 * arc, -0.42);
            // A soft ninth expands the harmony; dominant A stays pointed and clear.
            if root != 57 && !self.mood.ending() {
                self.play(0, root + 14, 8.5, 0.035 * arc, 0.38);
            }
            if battle || self.mood == Mood::Victory {
                self.play(
                    if e > 0.7 { 9 } else { 8 },
                    root,
                    8.5,
                    0.11 + e * 0.12,
                    0.25,
                );
            }
        }
        if t == 6 && phrase % 4 >= 2 && !self.mood.ending() {
            let next = BASSES[(phrase + 1) % 32];
            let step = if next > bass {
                bass + 2
            } else {
                bass.saturating_sub(2)
            };
            self.play(2, step, 5.7, 0.09 * arc, 0.24);
        }
        // Harp answers the piano on offbeats; it no longer doubles every attack.
        if (t == 3 || t == 9) && (!self.mood.ending() || self.ending_bars > 0) {
            let pitch = root + if t == 3 { 7 } else { 12 + third };
            self.play(if pitch < 70 { 6 } else { 7 }, pitch, 5.0, 0.10 * arc, -0.3);
        }
    }

    fn keyboard(&mut self, root: u8, third: u8, arc: f32) {
        let t = self.tick;
        let phrase = (self.bar % 32) as usize;
        let e = self.energy;
        let ending = self.mood.ending();
        if ending && self.ending_bars == 0 {
            if self.bar.is_multiple_of(4) && t == 0 {
                self.piano(root, 14.0, 0.16, 0.18);
                self.piano(root + 7, 14.0, 0.10, 0.15);
            }
            return;
        }
        let bass = if ending {
            root - 24
        } else {
            BASSES[phrase] - 12
        };
        let touch = (0.22 + 0.15 * arc + 0.48 * e).min(1.0);
        if t == 0 || t == 6 {
            self.piano(
                if t == 0 { bass } else { bass + 12 },
                8.5,
                0.22 * arc,
                touch * 0.78,
            );
        }
        // Four distinct two-hand accompaniment patterns across the 32-bar arc.
        // Delayed inner voices and held pedal tones give the melody room.
        let pattern = match phrase / 8 {
            0 => [0, 255, 7, 255, 12 + third, 255, 0, 255, 14, 255, 7, 255],
            1 => [
                255,
                7,
                255,
                12 + third,
                255,
                14,
                255,
                7,
                255,
                12,
                255,
                third,
            ],
            2 => [0, 255, third, 7, 255, 12, 0, 255, 7, third, 255, 14],
            _ => [0, 255, 7, 255, 12 + third, 14, 255, 12, 255, 7, 255, third],
        };
        let degree = pattern[usize::from(t)];
        if degree != 255 {
            let pitch = root - 12 + degree;
            let accent = if t.is_multiple_of(6) { 1.0 } else { 0.74 };
            self.piano(pitch, 5.2, 0.17 * arc * accent, touch);
        }
        // Suspension resolves within the bar; late phrases grow wider, not just louder.
        if t == 1 && phrase % 4 == 2 && !ending {
            self.piano(root + 5, 2.0, 0.085 * arc, touch * 0.8);
        }
        if t == 4 && phrase % 4 == 2 && !ending {
            self.piano(root + third, 5.0, 0.10 * arc, touch * 0.8);
        }
        if t == 10 && phrase % 8 >= 6 && !ending {
            self.piano(root + 19, 4.2, 0.08 * arc, touch * 0.72);
        }
    }

    fn theme(&mut self, root: u8, third: u8, arc: f32) {
        let t = self.tick;
        let phrase = (self.bar % 32) as usize;
        let e = self.energy;
        if !self.mood.ending() {
            let (pitch, length) = MELODY[phrase][usize::from(t)];
            if pitch > 0 {
                // The piano carries the theme in every state. Horns answer only
                // at the peak of battle, leaving its transients and phrasing clear.
                self.piano(
                    pitch,
                    f32::from(length) + 2.0,
                    0.27 * arc + e * 0.035,
                    0.36 + e * 0.5,
                );
                if self.mood == Mood::Battle && e > 0.55 && t.is_multiple_of(6) {
                    self.play(8, pitch - 12, f32::from(length) * 0.9, 0.10 * e, 0.18);
                }
            } else if phrase < 4 && (t == 2 || t == 8) {
                self.piano(
                    root + if t == 2 { 7 } else { 12 + third },
                    6.5,
                    0.18 * arc,
                    0.24,
                );
            }
            // Contrary-motion cello/viola dialogue develops on alternate cycles.
            if phrase % 8 >= 4 && (t == 3 || t == 9) {
                let degree = if t == 3 { 7 } else { third };
                self.play(
                    if (self.bar / 32).is_multiple_of(2) {
                        2
                    } else {
                        1
                    },
                    root + degree,
                    4.4,
                    0.105 * arc,
                    0.32,
                );
            }
            if phrase % 8 == 7 && t == 10 {
                self.piano(CHORDS[(phrase + 1) % 32].0 + 7, 3.0, 0.105, 0.32);
            }
        } else if self.ending_bars > 0 && (t == 0 || t == 6) {
            let rise = 4 - self.ending_bars;
            let pitch = match self.mood {
                Mood::Victory => root + if t == 0 { 7 } else { 12 + third },
                Mood::Defeat => root + if t == 0 { 7 } else { third },
                _ => root + if rise < 2 { 7 } else { 12 },
            };
            self.piano(
                pitch,
                8.0,
                0.25,
                if self.mood == Mood::Victory {
                    0.65
                } else {
                    0.23
                },
            );
            if self.mood == Mood::Victory {
                self.play(8, pitch - 12, 5.5, 0.13, 0.2);
            }
        }
    }

    fn percussion(&mut self, root: u8, third: u8) {
        let t = self.tick;
        let e = self.energy;
        let battle = self.mood == Mood::Battle;
        if t == 0 && ((battle && e > 0.25) || (self.ending_bars == 4 && self.mood == Mood::Victory))
        {
            self.play(11, root - 24, 4.0, 0.12 + e * 0.22, 0.0);
            if self.bar.is_multiple_of(4) || self.mood.ending() {
                self.play(14, 60, 10.0, 0.045 + e * 0.07, 0.3);
            }
        }
        if battle && e > 0.18 && (t.is_multiple_of(2) || e > 0.78) {
            let pitch = root + [0, 7, 12, 7, third, 7][usize::from(t) % 6];
            self.play(
                3 + ((self.bar + u64::from(t)) % 2) as usize,
                pitch,
                0.75,
                (0.06 + 0.16 * e) * if t.is_multiple_of(6) { 1.2 } else { 0.8 },
                -0.45,
            );
        }
        if battle && e > 0.4 && (t == 5 || t == 11 || (e > 0.8 && t == 10)) {
            self.play(
                12 + (self.bar % 2) as usize,
                60,
                0.8,
                0.045 + e * 0.11,
                0.12,
            );
        }
        if battle && e > 0.6 && t == 6 {
            self.play(11, root - 17, 3.0, 0.18 * e, 0.0);
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

// Shared bass tones/inversions connect neighbouring harmonies without a reset.
const BASSES: [u8; 32] = [
    50, 53, 52, 49, 50, 46, 45, 48, 50, 47, 46, 49, 50, 53, 53, 52, 43, 46, 49, 50, 45, 48, 50, 46,
    45, 48, 46, 49, 46, 48, 49, 45,
];

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
