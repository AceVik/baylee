//! Five original suites in B♭ Dorian, performed by one persistent sampler.
//! Composition is in `manuscript`; arranging is a pure, bounded note scheduler.
//! Scene/theme changes are admitted on the next
//! eighth (at most half a second). Result cues finish before leaving the result.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
// Bounded MIDI, tick and audio-frame arithmetic.
use super::{
    RATE, ScoreRequest, bank,
    orchestra::{BLOCK, Orchestra, Touch},
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
mod arrangement;
mod manuscript;
mod part;
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
    /// Arrangement currently performed.
    pub movement: Movement,
    /// Absolute bars completed; never reset by a scene change.
    pub bar: u64,
    /// Bar in this arrangement, including the initial cue for endings.
    pub phrase_bar: u32,
}

/// Endless stereo performance with bounded polyphony and no render-time allocation.
/// Held notes and the room survive scene changes, as does the sample clock.
pub struct Tune {
    control: Arc<ScoreControl>,
    orchestra: Orchestra,
    position: Position,
    tick: u8,
    until_tick: f64,
    eighth: f64,
    target: f64,
    started: bool,
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
            position: Position {
                theme: request.theme,
                movement,
                bar: 0,
                phrase_bar: 0,
            },
            tick: 0,
            until_tick: 0.0,
            eighth,
            target: eighth,
            started: false,
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
        let request = self.control.get();
        let movement = Movement::wanted(request, self.position.movement);
        let protected =
            self.position.movement.ending() && self.position.phrase_bar == 0 && self.tick < 4;
        let urgent = movement.ending() && movement != self.position.movement;
        if !self.started || (!protected || urgent) {
            if movement != self.position.movement || request.theme != self.position.theme {
                self.orchestra.release_notes();
                self.position.movement = movement;
                self.position.theme = request.theme;
                self.position.phrase_bar = 0;
                self.tick = 0;
            }
            self.target = self.position.theme.eighth(movement);
        }
        self.started = true;
    }

    fn transport(&mut self) {
        if self.until_tick > 0.0 {
            return;
        }
        self.admit();
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
            if note.instrument == arrangement::Instrument::Lyre {
                self.orchestra
                    .lute(note.pitch, length, note.gain, note.instrument.pan(), 0.58);
            } else {
                let family = match note.instrument {
                    arrangement::Instrument::Harp => bank::HARP,
                    arrangement::Instrument::Zither => bank::PSALTERY,
                    arrangement::Instrument::Violin => bank::VIOLIN,
                    arrangement::Instrument::Viola => bank::VIOLAS,
                    arrangement::Instrument::Cello => bank::CELLO,
                    arrangement::Instrument::Bass => bank::CONTRABASS,
                    arrangement::Instrument::Trombone => bank::TROMBONE,
                    arrangement::Instrument::ViolinShort => bank::VIOLINS_SPIC,
                    arrangement::Instrument::ViolaShort => bank::VIOLAS_SPIC,
                    arrangement::Instrument::CelloShort => bank::CELLOS_SPIC,
                    arrangement::Instrument::Lyre => unreachable!("modelled string above"),
                };
                self.orchestra.note(
                    family.nearest(note.pitch),
                    note.pitch,
                    length,
                    note.gain,
                    touch,
                );
            }
        }
        self.tick += 1;
        if self.tick == self.position.theme.ticks() {
            self.tick = 0;
            self.position.bar += 1;
            self.position.phrase_bar = self.position.phrase_bar.saturating_add(1);
        }
        self.until_tick += f64::from(RATE) * self.eighth;
    }
    fn advance(&mut self) {
        self.eighth += (self.target - self.eighth) / (f64::from(RATE) * 3.0);
        self.until_tick -= 1.0;
    }
    /// Render one stereo frame, the scalar reference for the block renderer.
    pub fn frame(&mut self) -> [f32; 2] {
        self.transport();
        self.advance();
        self.orchestra.frame()
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
mod tests;
