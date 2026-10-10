//! Recorded orchestra and our modelled lyre, sharing one continuous room.
//! Studio resampling is prepared before playback; both banks render allocation-free.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
// MIDI pitches, buffer indices and bounded audio amplitudes.
use super::{RATE, score::arrangement::Instrument};
mod sampled;
mod synth;
pub(super) use sampled::{Def, Family, Kind};
mod plucked;
use plucked::{Lute, STRINGS};
use sampled::Voice;

pub(super) fn prepare() {
    sampled::prepare();
    synth::prepare();
}
/// How a note is touched beyond its pitch, length and level.
#[derive(Clone, Copy, Debug)]
pub(super) struct Touch {
    /// Left -1 to right 1.
    pub pan: f32,
    /// Seconds to full level; `None` takes the instrument's own.
    pub attack: Option<f32>,
    /// Seconds to silence after the end; `None` takes the instrument's own.
    pub release: Option<f32>,
    /// Seconds after the tick it is asked on that it begins: heterophony's
    /// breath, an ornament's place.
    pub late: f32,
}

impl Touch {
    pub(super) const fn at(pan: f32) -> Self {
        Self {
            pan,
            attack: None,
            release: None,
            late: 0.0,
        }
    }
    pub(super) const fn attack(mut self, seconds: f32) -> Self {
        self.attack = Some(seconds);
        self
    }
    pub(super) const fn release(mut self, seconds: f32) -> Self {
        self.release = Some(seconds);
        self
    }
    pub(super) const fn late(mut self, seconds: f32) -> Self {
        self.late += seconds;
        self
    }
}

struct Delay {
    data: Vec<f32>,
    at: usize,
    low: f32,
}
impl Delay {
    fn new(frames: usize) -> Self {
        Self {
            data: vec![0.0; frames],
            at: 0,
            low: 0.0,
        }
    }
    fn comb(&mut self, input: f32) -> f32 {
        let out = self.data[self.at];
        // Damped: a warm room, not a bright one.
        self.low += (out - self.low) * 0.24;
        self.data[self.at] = input + self.low * 0.84;
        self.step();
        out
    }
    /// The next slot, round the ring: a compare, not a division per sample.
    fn step(&mut self) {
        self.at += 1;
        if self.at == self.data.len() {
            self.at = 0;
        }
    }
    fn diffuse(&mut self, input: f32) -> f32 {
        let old = self.data[self.at];
        let out = old - input * 0.5;
        self.data[self.at] = input + out * 0.5;
        self.step();
        out
    }
}

/// Maximum block size; also the size of the preallocated room scratch.
pub(super) const BLOCK: usize = 256;
/// Fixed voice budget. The score uses fewer than 40, including transition tails.
pub(super) const POLYPHONY: usize = 96;
const WET: f32 = 0.38;
const PLUCK_ROOM: f32 = 0.18;

pub(super) struct Orchestra {
    voices: Vec<Voice>,
    synth: synth::Synth,
    strings: [Lute; STRINGS],
    plucks: u32,
    room: [Delay; 8],
    scatter: [Delay; 4],
    tilt: [f32; 2],
}
impl Default for Orchestra {
    fn default() -> Self {
        prepare();
        Self {
            voices: Vec::with_capacity(POLYPHONY),
            synth: synth::Synth::default(),
            strings: std::array::from_fn(|_| Lute::new()),
            plucks: 0,
            room: [1631, 2043, 2327, 2821, 1697, 2101, 2399, 2921].map(Delay::new),
            scatter: [379, 123, 413, 139].map(Delay::new),
            tilt: [0.0; 2],
        }
    }
}
fn panned(gain: f32, pan: f32) -> [f32; 2] {
    [
        ((1.0 - pan) * 0.5).sqrt() * gain,
        f32::midpoint(1.0, pan).sqrt() * gain,
    ]
}
impl Orchestra {
    #[cfg(test)]
    pub(super) fn note(
        &mut self,
        instrument: Instrument,
        pitch: u8,
        seconds: f32,
        gain: f32,
        touch: Touch,
    ) {
        self.note_with_samples(
            instrument,
            pitch,
            seconds,
            gain,
            touch,
            super::SampleSet::Studio48,
        );
    }
    pub(super) fn note_with_samples(
        &mut self,
        instrument: Instrument,
        pitch: u8,
        seconds: f32,
        gain: f32,
        touch: Touch,
        samples: super::SampleSet,
    ) {
        if instrument.synthetic() {
            self.synth
                .note(instrument, pitch, seconds, panned(gain, touch.pan), touch);
        } else if instrument == Instrument::Lyre {
            // A free voice normally exists; deterministic stealing is a last resort.
            let slot = self
                .strings
                .iter()
                .position(|s| !s.sounding)
                .unwrap_or_else(|| {
                    self.strings
                        .iter()
                        .enumerate()
                        .max_by_key(|(_, s)| s.age)
                        .map_or(0, |(i, _)| i)
                });
            self.plucks = self.plucks.wrapping_add(1);
            self.strings[slot].pluck(
                pitch,
                seconds,
                panned(gain * 1.8, touch.pan),
                0.80,
                0.9955,
                self.plucks,
            );
        } else if self.voices.len() < POLYPHONY {
            self.voices.push(Voice::new(
                instrument,
                pitch,
                seconds,
                panned(gain, touch.pan),
                touch,
                samples,
            ));
        }
    }
    /// Release from the current level (including notes already releasing).
    /// Never rewind a release envelope when a second change follows quickly.
    pub(super) fn release_notes(&mut self) {
        self.synth.release();
        for voice in &mut self.voices {
            voice.release();
        }
        for string in &mut self.strings {
            string.release();
        }
    }
    fn finish(&mut self, dry: [f32; 2], feed: f32) -> [f32; 2] {
        let mut wet = [0.0; 2];
        for (i, delay) in self.room.iter_mut().enumerate() {
            wet[i / 4] += delay.comb(feed) * 0.25;
        }
        for (i, delay) in self.scatter.iter_mut().enumerate() {
            wet[i / 2] = delay.diffuse(wet[i / 2]);
        }
        std::array::from_fn(|i| {
            let input = dry[i] + wet[i] * WET;
            self.tilt[i] += (input - self.tilt[i]) * 0.29;
            let x = (input - 0.25 * (input - self.tilt[i])) * 0.92;
            // Linear throughout the score's normal range; no always-on distortion.
            // The safety knee only catches pathological voice pileups above -1.4 dB.
            let over = x.abs() - 0.85;
            if over <= 0.0 {
                x
            } else {
                x.signum() * (0.85 + 0.15 * over / (0.15 + over))
            }
        })
    }
    pub(super) fn frame(&mut self) -> [f32; 2] {
        let mut dry = [0.0; 2];
        let mut feed = 0.0;
        self.voices.retain_mut(|voice| {
            if let Some(frame) = voice.next() {
                for i in 0..2 {
                    dry[i] += frame[i];
                }
                feed += (frame[0] + frame[1]) * 0.16;
                true
            } else {
                false
            }
        });
        for string in &mut self.strings {
            if let Some(frame) = string.next() {
                for i in 0..2 {
                    dry[i] += frame[i];
                }
                feed += (frame[0] + frame[1]) * PLUCK_ROOM;
            }
        }
        self.synth.frame(&mut dry, &mut feed);
        self.finish(dry, feed)
    }
    /// Voice-major blocks, with exactly the reference's summation order.
    pub(super) fn render(&mut self, out: &mut [[f32; 2]], feed: &mut [f32]) {
        let feed = &mut feed[..out.len()];
        out.fill([0.0; 2]);
        feed.fill(0.0);
        self.voices.retain_mut(|voice| {
            for (frame, send) in out.iter_mut().zip(feed.iter_mut()) {
                let Some(note) = voice.next() else {
                    return false;
                };
                for i in 0..2 {
                    frame[i] += note[i];
                }
                *send += (note[0] + note[1]) * 0.16;
            }
            true
        });
        for string in &mut self.strings {
            if !string.sounding {
                continue;
            }
            for (frame, send) in out.iter_mut().zip(feed.iter_mut()) {
                let Some(note) = string.next() else {
                    break;
                };
                for i in 0..2 {
                    frame[i] += note[i];
                }
                *send += (note[0] + note[1]) * PLUCK_ROOM;
            }
        }
        self.synth.render(out, feed);
        for (frame, send) in out.iter_mut().zip(feed.iter()) {
            *frame = self.finish(*frame, *send);
        }
    }
}
#[cfg(test)]
mod tests;
