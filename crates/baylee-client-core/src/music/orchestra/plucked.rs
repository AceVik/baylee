//! The allocated-once plucked-string voice.
use crate::music::RATE;

/// The longest delay a [`Lute`] string holds: a string down to 22 Hz.
const STRING: usize = 2048;
/// How many modelled strings ring at once; a further pluck takes the oldest.
pub(super) const STRINGS: usize = 24;

/// A plucked string synthesised by Karplus–Strong: a delay line of one
/// period, filled with a burst of filtered noise, fed back through a two-tap
/// average. Original excitation shared by the lyre, harp and zither models.
/// Its buffer is allocated once, with the orchestra. All excitation is original.
pub(super) struct Lute {
    line: Box<[f32; STRING]>,
    /// The delay in whole frames, and the fractional rest tuned by a
    /// first-order all-pass.
    period: usize,
    allpass: f32,
    decay: f32,
    allpass_in: f32,
    allpass_out: f32,
    at: usize,
    pub(super) age: u32,
    hold: u32,
    release: u32,
    gain: [f32; 2],
    pub(super) sounding: bool,
}

impl Lute {
    pub(super) fn new() -> Self {
        Self {
            line: Box::new([0.0; STRING]),
            period: 2,
            allpass: 0.0,
            decay: 0.998,
            allpass_in: 0.0,
            allpass_out: 0.0,
            at: 0,
            age: 0,
            hold: 0,
            release: 1,
            gain: [0.0; 2],
            sounding: false,
        }
    }

    /// Plucks it: a burst of noise from `seed`, darkened by `bright`.
    pub(super) fn pluck(
        &mut self,
        pitch: u8,
        seconds: f32,
        gain: [f32; 2],
        bright: f32,
        decay: f32,
        seed: u32,
    ) {
        self.decay = decay;
        let frequency = 440.0 * 2.0_f32.powf((f32::from(pitch) - 69.0) / 12.0);
        // The loop reads the oldest slot and averages it with the next
        // (newer) one: N slots and the average are N − ½ frames, so the line
        // holds the period plus a half, the all-pass the fraction.
        let delay = (RATE as f32 / frequency + 0.5).clamp(2.0, (STRING - 1) as f32);
        self.period = delay as usize;
        let rest = delay - self.period as f32;
        self.allpass = (1.0 - rest) / (1.0 + rest);
        self.allpass_in = 0.0;
        self.allpass_out = 0.0;
        let mut state = seed.wrapping_mul(0x9E37_79B9) | 1;
        let mut low = 0.0f32;
        for (i, slot) in self.line[..self.period].iter_mut().enumerate() {
            // xorshift32: deterministic, no allocation, no global state.
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let noise = (state as f32 / u32::MAX as f32) * 2.0 - 1.0;
            low += (noise - low) * (1.0 - bright);
            *slot =
                low * 0.28 + (std::f32::consts::TAU * i as f32 / self.period as f32).sin() * 0.34;
        }
        self.at = 0;
        self.age = 0;
        self.hold = (seconds * RATE as f32) as u32;
        self.release = (0.25 * RATE as f32) as u32;
        self.gain = gain;
        self.sounding = true;
    }

    /// Shorten a tail without changing its current envelope or phase.
    pub(super) fn release(&mut self) {
        let remaining = self.hold.saturating_sub(self.age) + self.release;
        if remaining > (RATE as f32 * 0.32) as u32 && self.age < self.hold {
            self.hold = self.age;
            self.release = (RATE as f32 * 0.32) as u32;
        }
    }

    /// One frame of the string, or `None` once it has died away.
    #[inline]
    pub(super) fn next(&mut self) -> Option<[f32; 2]> {
        if !self.sounding {
            return None;
        }
        if self.age >= self.hold + self.release {
            self.sounding = false;
            return None;
        }
        let out = self.line[self.at];
        let after = self.line[if self.at + 1 == self.period {
            0
        } else {
            self.at + 1
        }];
        let averaged = self.decay * 0.5 * (out + after);
        let tuned = self.allpass * (averaged - self.allpass_out) + self.allpass_in;
        self.allpass_in = averaged;
        self.allpass_out = tuned;
        self.line[self.at] = tuned;
        self.at += 1;
        if self.at == self.period {
            self.at = 0;
        }
        let release =
            (1.0 - self.age.saturating_sub(self.hold) as f32 / self.release as f32).max(0.0);
        self.age += 1;
        let attack = (self.age as f32 / (RATE as f32 * 0.004)).min(1.0);
        Some(self.gain.map(|gain| gain * out * release * attack))
    }
}
