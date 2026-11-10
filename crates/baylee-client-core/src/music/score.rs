//! A public-domain memorial performance and fifteen original duel suites.
//! Composition is in `manuscript`; arranging is a pure, bounded note scheduler.
//! Scene/theme changes are admitted on the next
//! eighth (about 0.18–0.35 seconds). Result cues finish before leaving the result.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
// Bounded MIDI, tick and audio-frame arithmetic.
use super::{
    RATE, SampleSet, ScoreRequest,
    orchestra::{BLOCK, Orchestra, Touch},
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
pub(super) mod arrangement;
mod manuscript;
pub(in crate::music) mod memorial;
mod part;
mod styles;
mod themes;
pub use part::Movement;
pub use themes::Theme;

/// Lock-free message from the game to its musical conductor.
#[derive(Debug, Default)]
pub struct ScoreControl(AtomicU64);
impl ScoreControl {
    /// Set the desired scene and suite; invalid tension is sanitized by packing.
    pub fn set(&self, request: ScoreRequest) {
        self.0.store(request.pack(), Ordering::Relaxed);
    }
    /// Latest requested state (the audio conductor admits it on musical boundaries).
    #[must_use]
    pub fn get(&self) -> ScoreRequest {
        ScoreRequest::unpack(self.0.load(Ordering::Relaxed))
    }
}

/// The actual musical position, also returned by offline renders.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Position {
    /// Suite currently performed.
    pub theme: Theme,
    /// Bank currently admitted for new notes.
    pub samples: SampleSet,
    /// Arrangement currently performed.
    pub movement: Movement,
    /// Absolute bars (four-second observation windows in the memorial); never reset.
    pub bar: u64,
    /// Local bar/window in this arrangement, including the initial result cue.
    pub phrase_bar: u32,
}

/// Subtle, endless-loop tavern room murmur and hearth warmth for Lobby and Table.
struct TavernAmbience {
    seed: u32,
    env: f32,
    left_lp1: f32,
    left_lp2: f32,
    right_lp1: f32,
    right_lp2: f32,
    hearth_lp: f32,
    crackle_env: f32,
    cadence_a: f32,
    cadence_b: f32,
    cadence_c: f32,
}
impl Default for TavernAmbience {
    fn default() -> Self {
        Self {
            seed: 0x6D2B_79F5,
            env: 0.0,
            left_lp1: 0.0,
            left_lp2: 0.0,
            right_lp1: 0.0,
            right_lp2: 0.0,
            hearth_lp: 0.0,
            crackle_env: 0.0,
            cadence_a: 0.0,
            cadence_b: 1.7,
            cadence_c: 3.4,
        }
    }
}
impl TavernAmbience {
    fn next_noise(&mut self) -> f32 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        ((x >> 9) as f32) * (1.0 / 4_194_304.0) - 1.0
    }
    fn step(&mut self, active: bool) -> [f32; 2] {
        let target = if active { 1.0 } else { 0.0 };
        self.env += (target - self.env) * 0.0004;
        if self.env < 1e-5 {
            return [0.0; 2];
        }
        let n_l = self.next_noise();
        let n_r = self.next_noise();
        // Slow syllable-rate modulation (2.4 Hz, 3.7 Hz, 5.1 Hz) for distant warm tavern murmur.
        self.cadence_a = (self.cadence_a + std::f32::consts::TAU * 2.4 / RATE as f32)
            .rem_euclid(std::f32::consts::TAU);
        self.cadence_b = (self.cadence_b + std::f32::consts::TAU * 3.7 / RATE as f32)
            .rem_euclid(std::f32::consts::TAU);
        self.cadence_c = (self.cadence_c + std::f32::consts::TAU * 0.43 / RATE as f32)
            .rem_euclid(std::f32::consts::TAU);
        let mod_l = (0.55 + 0.30 * self.cadence_a.sin() + 0.15 * self.cadence_c.cos()).max(0.1);
        let mod_r = (0.55 + 0.30 * self.cadence_b.cos() + 0.15 * self.cadence_c.sin()).max(0.1);
        // Band-pass between ~220 Hz and ~680 Hz (warm wood-hall vowel murmur, no high hiss).
        self.left_lp1 += (n_l * mod_l - self.left_lp1) * 0.078;
        self.left_lp2 += (self.left_lp1 - self.left_lp2) * 0.024;
        let murmur_l = self.left_lp1 - self.left_lp2;

        self.right_lp1 += (n_r * mod_r - self.right_lp1) * 0.084;
        self.right_lp2 += (self.right_lp1 - self.right_lp2) * 0.026;
        let murmur_r = self.right_lp1 - self.right_lp2;

        // Soft hearth warmth and occasional gentle ember pop on the left.
        self.hearth_lp += (n_l - self.hearth_lp) * 0.014;
        if (self.seed & 0x3FFF) == 0x1357 {
            self.crackle_env = 0.35;
        }
        self.crackle_env *= 0.985;
        let hearth = self.hearth_lp * 0.45 + self.left_lp1 * self.crackle_env;

        let gain = 0.0075 * self.env;
        [
            (murmur_l * 0.75 + murmur_r * 0.25 + hearth * 0.45) * gain,
            (murmur_r * 0.75 + murmur_l * 0.25 + hearth * 0.20) * gain,
        ]
    }
}

/// Endless stereo performance with bounded polyphony and no render-time allocation.
/// Held notes and the room survive scene changes, as does the sample clock.
pub struct Tune {
    control: Arc<ScoreControl>,
    orchestra: Orchestra,
    memorial: memorial::Performance,
    tavern_ambience: TavernAmbience,
    position: Position,
    tick: u8,
    until_tick: f64,
    eighth: f64,
    target: f64,
    started: bool,
    pending_result: Option<ScoreRequest>,
    ahead: Vec<[f32; 2]>,
    scratch: Box<[f32; BLOCK]>,
    ahead_read: usize,
    right: Option<f32>,
}
impl Default for Tune {
    fn default() -> Self {
        Self::new()
    }
}
impl Tune {
    /// Begin the first suite at the title screen.
    #[must_use]
    pub fn new() -> Self {
        Self::with_control(Arc::new(ScoreControl::default()))
    }
    /// Begin with the conductor's current request, without a spurious title bar.
    #[must_use]
    pub fn with_control(control: Arc<ScoreControl>) -> Self {
        let request = control.get();
        let movement = Movement::wanted(request, Movement::Title);
        let eighth = request.theme.eighth(movement);
        Self {
            control,
            orchestra: Orchestra::default(),
            memorial: memorial::Performance::default(),
            tavern_ambience: TavernAmbience::default(),
            position: Position {
                theme: request.theme,
                samples: request.samples,
                movement,
                bar: 0,
                phrase_bar: 0,
            },
            tick: 0,
            until_tick: 0.0,
            eighth,
            target: eighth,
            started: false,
            pending_result: None,
            ahead: vec![[0.0; 2]; BLOCK],
            scratch: Box::new([0.0; BLOCK]),
            ahead_read: BLOCK,
            right: None,
        }
    }
    /// The arrangement actually admitted by the conductor.
    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }

    fn admit(&mut self) {
        let request = self
            .pending_result
            .take()
            .unwrap_or_else(|| self.control.get());
        let movement = Movement::wanted(request, self.position.movement);
        let protected =
            self.position.movement.ending() && self.position.phrase_bar == 0 && self.tick < 4;
        let urgent = movement.ending() && movement != self.position.movement;
        if !self.started || (!protected || urgent) {
            if movement != self.position.movement || request.theme != self.position.theme {
                let continuous =
                    memorial::contains(movement) && memorial::contains(self.position.movement);
                if !continuous {
                    self.orchestra.release_notes();
                    self.position.phrase_bar = 0;
                    self.tick = 0;
                }
                self.position.movement = movement;
                self.position.theme = request.theme;
            }
            self.position.samples = request.samples;
            self.target = self.position.theme.eighth(movement);
        }
        self.started = true;
    }

    fn transport(&mut self) {
        // Observe brief result screens between musical pulses. Otherwise a
        // dismissal before the next pulse could erase the attention cue.
        let request = self.control.get();
        let wanted = Movement::wanted(request, self.position.movement);
        if wanted.ending() && wanted != self.position.movement {
            self.pending_result = Some(request);
        }
        if self.until_tick > 0.0 {
            return;
        }
        if self.tick == self.position.theme.ticks() {
            self.tick = 0;
            self.position.bar += 1;
            self.position.phrase_bar = self.position.phrase_bar.saturating_add(1);
        }
        self.admit();
        if memorial::contains(self.position.movement) {
            self.until_tick += f64::from(
                self.memorial
                    .play(&mut self.orchestra, self.position.samples),
            );
            return;
        }
        let notes = arrangement::notes(
            self.position.theme,
            self.position.movement,
            self.position.phrase_bar,
            self.tick,
        );
        for note in notes.as_slice() {
            let length = note.length * self.eighth as f32;
            let touch = Touch::at(note.instrument.pan())
                .late(note.late * self.eighth as f32)
                .attack(if note.instrument.short() {
                    0.006
                } else {
                    0.045
                })
                .release(if note.instrument.short() { 0.10 } else { 0.28 });
            self.orchestra.note_with_samples(
                note.instrument,
                note.pitch,
                length,
                note.gain,
                touch,
                self.position.samples,
            );
        }
        self.tick += 1;
        let swing = if self.position.theme == Theme::Lantern {
            if self.tick % 2 == 1 { 1.18 } else { 0.82 }
        } else {
            1.0
        };
        self.until_tick += f64::from(RATE) * self.eighth * swing;
    }
    fn advance(&mut self) {
        if memorial::contains(self.position.movement) && self.memorial.advance() {
            self.position.bar += 1;
            self.position.phrase_bar = self.position.phrase_bar.saturating_add(1);
        }
        self.eighth += (self.target - self.eighth) / (f64::from(RATE) * 3.0);
        self.until_tick -= 1.0;
    }
    /// Render one stereo frame, the scalar reference for the block renderer.
    pub fn frame(&mut self) -> [f32; 2] {
        self.transport();
        self.advance();
        let active = matches!(
            self.position.movement,
            Movement::Title | Movement::Lobby | Movement::Standard
        );
        let amb = self.tavern_ambience.step(active);
        let [l, r] = self.orchestra.frame();
        [l + amb[0], r + amb[1]]
    }
    /// Render a run of stereo frames, with exactly the scalar renderer's timing.
    pub fn render(&mut self, out: &mut [[f32; 2]]) {
        let mut done = 0;
        while done < out.len() {
            self.transport();
            let due = (self.until_tick.ceil() as usize).max(1);
            let run = due.min(out.len() - done).min(BLOCK);
            for _ in 0..run {
                self.advance();
            }
            self.orchestra
                .render(&mut out[done..done + run], &mut self.scratch[..]);
            let active = matches!(
                self.position.movement,
                Movement::Title | Movement::Lobby | Movement::Standard
            );
            for frame in &mut out[done..done + run] {
                let amb = self.tavern_ambience.step(active);
                frame[0] += amb[0];
                frame[1] += amb[1];
            }
            done += run;
        }
    }
}
impl Iterator for Tune {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if let Some(right) = self.right.take() {
            return Some(right);
        }
        if self.ahead_read == BLOCK {
            let mut ahead = std::mem::take(&mut self.ahead);
            self.render(&mut ahead);
            self.ahead = ahead;
            self.ahead_read = 0;
        }
        let [left, right] = self.ahead[self.ahead_read];
        self.ahead_read += 1;
        self.right = Some(right);
        Some(left)
    }
}
#[cfg(test)]
mod exploration_tests;
#[cfg(test)]
mod tests;
