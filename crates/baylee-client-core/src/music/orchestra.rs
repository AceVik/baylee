//! Sample playback and the shared concert room. The bank is decoded once;
//! rendering allocates nothing. See art/music/samples.json for provenance.
//!
//! Two ways to render, one sound: [`Orchestra::frame`] is the reference, one
//! stereo frame at a time, and [`Orchestra::render`] renders a run of frames
//! voice by voice and then delay line by delay line. Every sum is taken in the
//! same order as the reference takes it, so the two agree bit for bit
//! (`a_block_is_the_frames_it_replaces`); the block costs a fraction of the
//! audio thread (`docs/perf-client.md`), which was a third of the client's
//! CPU while the front door's score played.
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
            sample!("horn", 60),
            sample!("horn-forte", 60),
            sample!("oboe", 74),
            sample!("timpani", 42),
            sample!("snare1", 60),
            sample!("snare2", 60),
            sample!("cymbal", 60),
            sample!("piano-2-pp", 36),
            sample!("piano-2-mf", 36),
            sample!("piano-2-f", 36),
            sample!("piano-3-pp", 48),
            sample!("piano-3-mf", 48),
            sample!("piano-3-f", 48),
            sample!("piano-4-pp", 60),
            sample!("piano-4-mf", 60),
            sample!("piano-4-f", 60),
            sample!("piano-5-pp", 72),
            sample!("piano-5-mf", 72),
            sample!("piano-5-f", 72),
            sample!("piano-6-pp", 84),
            sample!("piano-6-mf", 84),
            sample!("piano-6-f", 84),
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
    room_send: f32,
}
impl Voice {
    /// Renders this voice into a run of frames, adding to `dry` and `feed`
    /// exactly what [`Voice::next`] would have added frame by frame, and
    /// answers whether it is still sounding.
    ///
    /// The envelope's three ratios are each exactly 1.0 for most of a note
    /// (after the attack, before the release, away from the sample's end),
    /// so they are only divided out where they are not: the same numbers,
    /// without three divisions per voice per frame.
    fn render(&mut self, bank: &[Instrument], dry: &mut [[f32; 2]], feed: &mut [f32]) -> bool {
        let samples = &bank[self.instrument].pcm;
        for (out, send) in dry.iter_mut().zip(feed.iter_mut()) {
            let i = self.position as usize;
            if i + 1 >= samples.len() || self.age >= self.hold + self.release {
                return false;
            }
            let fraction = self.position.fract() as f32;
            let wave = samples[i] + (samples[i + 1] - samples[i]) * fraction;
            let steady =
                self.age >= self.attack && self.age <= self.hold && samples.len() - i >= 1102;
            let frame = if steady {
                // Each ratio is exactly 1.0 here, and so is their product:
                // `gain * wave * 1.0` is `gain * wave`.
                self.gain.map(|gain| gain * wave)
            } else {
                let attack = (self.age as f32 / self.attack as f32).min(1.0);
                let release = (1.0
                    - self.age.saturating_sub(self.hold) as f32 / self.release as f32)
                    .max(0.0);
                let tail = ((samples.len() - i) as f32 / 1102.0).min(1.0);
                let envelope = attack * release * release * tail;
                self.gain.map(|gain| gain * wave * envelope)
            };
            self.age += 1;
            self.position += self.rate;
            out[0] += frame[0];
            out[1] += frame[1];
            *send += (frame[0] + frame[1]) * self.room_send;
        }
        true
    }

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

/// The most frames one [`Orchestra::render`] call takes.
pub(super) const BLOCK: usize = 256;

pub(super) struct Orchestra {
    voices: Vec<Voice>,
    room: [Delay; 8],
    scatter: [Delay; 4],
}
impl Default for Orchestra {
    fn default() -> Self {
        bank();
        Self {
            voices: Vec::with_capacity(112),
            room: [1499, 1877, 2137, 2593, 1559, 1931, 2203, 2683].map(Delay::new),
            scatter: [347, 113, 379, 127].map(Delay::new),
        }
    }
}
impl Orchestra {
    pub(super) fn note(&mut self, instrument: usize, pitch: u8, seconds: f32, gain: f32, pan: f32) {
        if self.voices.len() >= 112 {
            return;
        }
        let sample = &bank()[instrument];
        let plucked = (3..=7).contains(&instrument) || instrument >= 11;
        self.voices.push(Voice {
            instrument,
            position: 0.0,
            rate: 0.5 * 2.0_f64.powf((f64::from(pitch) - f64::from(sample.root)) / 12.0),
            age: 0,
            hold: (seconds * RATE as f32) as u32,
            release: ((if instrument >= 15 {
                2.8
            } else if plucked {
                1.8
            } else {
                0.85
            }) * RATE as f32) as u32,
            attack: if instrument >= 15 {
                44
            } else if plucked {
                100
            } else {
                2205
            },
            room_send: if instrument >= 15 { 0.09 } else { 0.22 },
            gain: [
                ((1.0 - pan) * 0.5).sqrt() * gain,
                f32::midpoint(1.0, pan).sqrt() * gain,
            ],
        });
    }
    /// Five recorded registers and three touch layers retain the piano's
    /// hammer character. Adjacent velocity layers blend in power, not tracks.
    pub(super) fn piano(&mut self, pitch: u8, seconds: f32, gain: f32, touch: f32) {
        let zone = usize::from((pitch.saturating_sub(30) / 12).min(4));
        let velocity = touch.clamp(0.0, 1.0) * 2.0;
        let layer = (velocity.floor() as usize).min(1);
        let mix = velocity - layer as f32;
        let pan = ((f32::from(pitch) - 60.0) / 60.0).clamp(-0.45, 0.45);
        for (offset, weight) in [(0, (1.0 - mix).sqrt()), (1, mix.sqrt())] {
            if weight > 0.001 {
                self.note(
                    15 + zone * 3 + layer + offset,
                    pitch,
                    seconds,
                    gain * weight,
                    pan,
                );
            }
        }
    }

    /// Renders `out.len()` frames: the same frames, bit for bit, as as many
    /// calls of [`Self::frame`] (`a_block_is_the_frames_it_replaces`).
    /// `feed` is scratch, at least as long as `out`.
    pub(super) fn render(&mut self, out: &mut [[f32; 2]], feed: &mut [f32]) {
        let feed = &mut feed[..out.len()];
        out.fill([0.0; 2]);
        feed.fill(0.0);
        let bank = bank();
        self.voices
            .retain_mut(|voice| voice.render(bank, out, feed));
        // The room, delay line by delay line: each frame still sums its four
        // lines per side in the reference's order, starting from zero.
        let mut wet = [[0.0f32; 2]; BLOCK];
        let wet = &mut wet[..out.len()];
        for (i, delay) in self.room.iter_mut().enumerate() {
            for (w, f) in wet.iter_mut().zip(feed.iter()) {
                w[i / 4] += delay.comb(*f) * 0.25;
            }
        }
        for (i, delay) in self.scatter.iter_mut().enumerate() {
            for w in wet.iter_mut() {
                w[i / 2] = delay.diffuse(w[i / 2]);
            }
        }
        for (frame, w) in out.iter_mut().zip(wet.iter()) {
            *frame = std::array::from_fn(|i| {
                let x = (frame[i] + w[i] * 0.48) * 1.5;
                x / (1.0 + x.abs())
            });
        }
    }

    /// One stereo frame: the reference [`Self::render`] is measured against.
    pub(super) fn frame(&mut self) -> [f32; 2] {
        let mut dry = [0.0; 2];
        let mut feed = 0.0;
        self.voices.retain_mut(|voice| {
            if let Some(frame) = voice.next(bank()) {
                dry[0] += frame[0];
                dry[1] += frame[1];
                feed += (frame[0] + frame[1]) * voice.room_send;
                true
            } else {
                false
            }
        });
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The block renderer is the frame renderer, bit for bit: the same notes
    /// started at the same frames, rendered both ways over many runs of
    /// uneven length, give identical samples — through attacks, releases,
    /// sample ends and the room's whole delay ring.
    #[test]
    #[allow(clippy::float_cmp)] // bit-identity is the claim
    fn a_block_is_the_frames_it_replaces() {
        let mut by_frame = Orchestra::default();
        let mut by_block = Orchestra::default();
        let mut feed = [0.0f32; BLOCK];
        let mut block = [[0.0f32; 2]; BLOCK];
        // Uneven runs, so a run boundary lands everywhere relative to a
        // note's envelope and every delay line's ring.
        let runs = [1usize, 7, 256, 13, 100, 256, 3, 199, 256, 64];
        let mut rendered = 0usize;
        for round in 0..400usize {
            if round % 3 == 0 {
                // A note on every instrument in turn, long and short.
                let instrument = round % bank().len();
                let pitch = 40 + (round % 40) as u8;
                let seconds = if round % 2 == 0 { 0.05 } else { 1.3 };
                for orchestra in [&mut by_frame, &mut by_block] {
                    orchestra.note(instrument, pitch, seconds, 0.4, 0.2);
                    orchestra.piano(pitch, seconds, 0.3, 0.6);
                }
            }
            let run = runs[round % runs.len()];
            by_block.render(&mut block[..run], &mut feed);
            for (k, frame) in block[..run].iter().enumerate() {
                assert_eq!(*frame, by_frame.frame(), "frame {}", rendered + k);
            }
            rendered += run;
        }
        assert!(
            rendered > RATE as usize,
            "{rendered} frames is under a second"
        );
    }
}
