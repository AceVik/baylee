//! Sample playback and the shared concert room. The bank is decoded once;
//! rendering allocates nothing. See art/music/samples.json for provenance.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)] // bounded sample indices and audio amplitudes
use super::RATE;
use std::sync::OnceLock;

pub(super) struct Instrument {
    pcm: Vec<f32>,
    root: u8,
}

pub(super) fn bank() -> &'static [Instrument] {
    static BANK: OnceLock<Vec<Instrument>> = OnceLock::new();
    BANK.get_or_init(|| {
        macro_rules! sample {
            ($name:literal, $root:literal) => {
                Instrument {
                    pcm: include_bytes!(concat!("../../assets/orchestra/", $name, ".pcm"))
                        .chunks_exact(2)
                        .map(|s| f32::from(i16::from_le_bytes([s[0], s[1]])) / 32768.0)
                        .collect(),
                    root: $root,
                }
            };
        }
        vec![
            sample!("violins", 72),
            sample!("violas", 72),
            sample!("cellos", 60),
            sample!("spiccato1", 72),
            sample!("spiccato2", 72),
            sample!("harp-low", 48),
            sample!("harp-mid", 62),
            sample!("harp-high", 72),
            sample!("flute", 72),
            sample!("horn", 60),
            sample!("horn-forte", 60),
            sample!("oboe", 74),
            sample!("timpani", 42),
            sample!("snare1", 60),
            sample!("snare2", 60),
            sample!("cymbal", 60),
        ]
    })
}

struct Voice {
    instrument: usize,
    position: f64,
    rate: f64,
    age: u32,
    hold: u32,
    release: u32,
    attack: u32,
    gain: [f32; 2],
}
impl Voice {
    fn next(&mut self, bank: &[Instrument]) -> Option<[f32; 2]> {
        let samples = &bank[self.instrument].pcm;
        let i = self.position as usize;
        if i + 1 >= samples.len() || self.age >= self.hold + self.release {
            return None;
        }
        let fraction = self.position.fract() as f32;
        let wave = samples[i] + (samples[i + 1] - samples[i]) * fraction;
        let attack = (self.age as f32 / self.attack as f32).min(1.0);
        let release =
            (1.0 - self.age.saturating_sub(self.hold) as f32 / self.release as f32).max(0.0);
        let tail = ((samples.len() - i) as f32 / 1102.0).min(1.0);
        let envelope = attack * release * release * tail;
        self.age += 1;
        self.position += self.rate;
        Some(self.gain.map(|gain| gain * wave * envelope))
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
        self.low += (out - self.low) * 0.34;
        self.data[self.at] = input + self.low * 0.84;
        self.at = (self.at + 1) % self.data.len();
        out
    }
    fn diffuse(&mut self, input: f32) -> f32 {
        let old = self.data[self.at];
        let out = old - input * 0.5;
        self.data[self.at] = input + out * 0.5;
        self.at = (self.at + 1) % self.data.len();
        out
    }
}

pub(super) struct Orchestra {
    voices: Vec<Voice>,
    room: [Delay; 8],
    scatter: [Delay; 4],
}
impl Default for Orchestra {
    fn default() -> Self {
        bank();
        Self {
            voices: Vec::with_capacity(80),
            room: [1499, 1877, 2137, 2593, 1559, 1931, 2203, 2683].map(Delay::new),
            scatter: [347, 113, 379, 127].map(Delay::new),
        }
    }
}
impl Orchestra {
    pub(super) fn note(&mut self, instrument: usize, pitch: u8, seconds: f32, gain: f32, pan: f32) {
        if self.voices.len() >= 80 {
            return;
        }
        let sample = &bank()[instrument];
        let plucked = (3..=7).contains(&instrument) || instrument >= 12;
        self.voices.push(Voice {
            instrument,
            position: 0.0,
            rate: 0.5 * 2.0_f64.powf((f64::from(pitch) - f64::from(sample.root)) / 12.0),
            age: 0,
            hold: (seconds * RATE as f32) as u32,
            release: ((if plucked { 1.8 } else { 0.85 }) * RATE as f32) as u32,
            attack: if plucked { 100 } else { 2205 },
            gain: [
                ((1.0 - pan) * 0.5).sqrt() * gain,
                f32::midpoint(1.0, pan).sqrt() * gain,
            ],
        });
    }
    pub(super) fn frame(&mut self) -> [f32; 2] {
        let mut dry = [0.0; 2];
        self.voices.retain_mut(|voice| {
            if let Some(frame) = voice.next(bank()) {
                dry[0] += frame[0];
                dry[1] += frame[1];
                true
            } else {
                false
            }
        });
        let feed = (dry[0] + dry[1]) * 0.18;
        let mut wet = [0.0; 2];
        for (i, delay) in self.room.iter_mut().enumerate() {
            wet[i / 4] += delay.comb(feed) * 0.25;
        }
        for (i, delay) in self.scatter.iter_mut().enumerate() {
            wet[i / 2] = delay.diffuse(wet[i / 2]);
        }
        std::array::from_fn(|i| {
            let x = (dry[i] + wet[i] * 0.48) * 1.5;
            // Soft safety ceiling leaves room for UI cues; never hard clips.
            x / (1.0 + x.abs())
        })
    }
}
