//! Original, band-limited additive patches and synthesized percussion, native 48 kHz.
//! Tables and pitch increments are prepared outside the callback. No samples,
//! presets, loops or melodies from a third-party synthesizer are embedded.
use super::{Instrument, RATE, Touch};
use std::sync::OnceLock;
const SIZE: usize = 2048;
const TONES: usize = 11;
const VOICES: usize = 64;

struct Timbre {
    bright: Box<[f32]>,
    body: Box<[f32]>,
    attack: f32,
    decay: f32,
    colour_decay: f32,
    room: f32,
}
struct Bank {
    tones: Vec<Timbre>,
    drums: [Box<[f32]>; 3],
    steps: [f64; 128],
}
fn wave(partials: &[f32; 12]) -> Box<[f32]> {
    // Highest score pitch is 88 (1319 Hz): even partial 12 stays below 16 kHz.
    // Sum rather than naive saw/pulse discontinuities, which alias when pitched up.
    let scale = 0.70 / partials.iter().map(|v| v.abs()).sum::<f32>();
    (0..=SIZE)
        .map(|i| {
            let phase = (i % SIZE) as f64 / SIZE as f64 * std::f64::consts::TAU;
            partials
                .iter()
                .enumerate()
                .map(|(n, gain)| (phase * (n + 1) as f64).sin() as f32 * gain)
                .sum::<f32>()
                * scale
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}
#[allow(clippy::too_many_lines)] // Original spectral/envelope presets, reviewed as a table.
fn timbre(instrument: Instrument) -> Timbre {
    use Instrument::{
        AnalogLead, Bell, ChipLead, Clav, ElectricPiano, Flute, Guitar, Marimba, Pad, Piano,
        SynthBass,
    };
    let (partials, attack, decay, colour, room) = match instrument {
        Piano => (
            [
                1.0, 0.55, 0.24, 0.18, 0.08, 0.05, 0.03, 0.02, 0.01, 0.0, 0.0, 0.0,
            ],
            0.004,
            0.58,
            3.5,
            0.12,
        ),
        Clav => (
            [
                1.0, 0.70, 0.55, 0.36, 0.25, 0.18, 0.10, 0.07, 0.04, 0.03, 0.02, 0.01,
            ],
            0.002,
            1.3,
            2.5,
            0.08,
        ),
        Flute => (
            [
                1.0, 0.06, 0.20, 0.02, 0.04, 0.0, 0.01, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
            0.024,
            0.04,
            1.8,
            0.12,
        ),
        Guitar => (
            [
                1.0, 0.40, 0.30, 0.14, 0.04, 0.10, 0.02, 0.015, 0.01, 0.0, 0.0, 0.0,
            ],
            0.003,
            1.1,
            6.0,
            0.07,
        ),
        ElectricPiano => (
            [
                1.0, 0.10, 0.56, 0.015, 0.05, 0.0, 0.13, 0.0, 0.015, 0.0, 0.0, 0.0,
            ],
            0.006,
            0.72,
            5.0,
            0.09,
        ),
        AnalogLead => (
            [
                1.0, 0.45, 0.25, 0.14, 0.08, 0.05, 0.03, 0.02, 0.014, 0.01, 0.007, 0.005,
            ],
            0.012,
            0.09,
            7.0,
            0.07,
        ),
        ChipLead => (
            [
                1.0, 0.0, 0.333, 0.0, 0.20, 0.0, 0.143, 0.0, 0.111, 0.0, 0.091, 0.0,
            ],
            0.003,
            0.14,
            0.0,
            0.015,
        ),
        Marimba => (
            [1.0, 0.0, 0.0, 0.50, 0.0, 0.0, 0.0, 0.0, 0.0, 0.13, 0.0, 0.0],
            0.002,
            2.8,
            13.0,
            0.12,
        ),
        Bell => (
            [
                1.0, 0.28, 0.48, 0.0, 0.13, 0.18, 0.0, 0.08, 0.0, 0.0, 0.0, 0.0,
            ],
            0.004,
            0.72,
            1.8,
            0.24,
        ),
        Pad => (
            [
                1.0, 0.16, 0.22, 0.07, 0.10, 0.03, 0.04, 0.0, 0.01, 0.0, 0.0, 0.0,
            ],
            0.20,
            0.012,
            0.9,
            0.28,
        ),
        SynthBass => (
            [
                1.0, 0.24, 0.12, 0.06, 0.03, 0.01, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
            0.009,
            0.35,
            7.0,
            0.015,
        ),
        _ => unreachable!("melodic synthetic palette"),
    };
    let mut body = partials;
    for (i, partial) in body.iter_mut().enumerate() {
        if instrument != ChipLead {
            *partial /= (i + 1) as f32;
        }
    }
    Timbre {
        bright: wave(&partials),
        body: wave(&body),
        attack,
        decay,
        colour_decay: colour,
        room,
    }
}
fn drum(kind: usize) -> Box<[f32]> {
    let duration = [0.48, 0.28, 0.09][kind];
    let mut seed = 0x59ac_401d_u32;
    let mut low = 0.0;
    let mut phase = 0.0;
    (0..(duration * RATE as f32) as usize)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let noise = seed as f32 / u32::MAX as f32 * 2.0 - 1.0;
            low += (noise - low) * 0.30;
            let raw = match kind {
                0 => {
                    phase +=
                        std::f32::consts::TAU * (47.0 + 95.0 * (-t * 45.0).exp()) / RATE as f32;
                    phase.sin() * (-t * 12.0).exp()
                }
                1 => {
                    ((std::f32::consts::TAU * 178.0 * t).sin() * 0.32 + low * 1.2)
                        * (-t * 22.0).exp()
                }
                _ => (noise - low) * (-t * 65.0).exp() * 0.6,
            };
            raw * (t / 0.0015).min(1.0) * ((duration - t) / 0.006).min(1.0) * 0.75
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}
fn bank() -> &'static Bank {
    static BANK: OnceLock<Bank> = OnceLock::new();
    BANK.get_or_init(|| Bank {
        tones: [
            Instrument::Piano,
            Instrument::Clav,
            Instrument::Flute,
            Instrument::Guitar,
            Instrument::ElectricPiano,
            Instrument::AnalogLead,
            Instrument::ChipLead,
            Instrument::Marimba,
            Instrument::Bell,
            Instrument::Pad,
            Instrument::SynthBass,
        ]
        .map(timbre)
        .into(),
        drums: std::array::from_fn(drum),
        steps: std::array::from_fn(|pitch| {
            440.0 * 2.0_f64.powf((pitch as f64 - 69.0) / 12.0) * SIZE as f64 / f64::from(RATE)
        }),
    })
}
pub(super) fn prepare() {
    bank();
}
fn read(table: &[f32], phase: f64) -> f32 {
    let at = phase as usize;
    let mix = (phase - at as f64) as f32;
    table[at] + (table[at + 1] - table[at]) * mix
}
struct Voice {
    index: usize,
    phase: f64,
    step: f64,
    age: u32,
    wait: u32,
    hold: u32,
    gain: [f32; 2],
    attack: f32,
    envelope: f32,
    decay: f32,
    colour: f32,
    colour_decay: f32,
    release: f32,
    releasing: bool,
    room: f32,
}
impl Voice {
    fn new(instrument: Instrument, pitch: u8, seconds: f32, gain: [f32; 2], touch: Touch) -> Self {
        let index = instrument as usize - Instrument::Piano as usize;
        let tone = bank().tones.get(index);
        let release = if instrument == Instrument::Pad {
            0.40
        } else {
            0.12
        };
        Self {
            index,
            phase: 0.0,
            step: bank().steps[usize::from(pitch.clamp(28, 88))],
            age: 0,
            wait: (touch.late * RATE as f32) as u32,
            hold: (seconds * RATE as f32) as u32,
            gain,
            attack: tone.map_or(1.0, |t| t.attack * RATE as f32),
            envelope: 1.0,
            decay: tone.map_or(1.0, |t| (-t.decay / RATE as f32).exp()),
            colour: 1.0,
            colour_decay: tone.map_or(1.0, |t| (-t.colour_decay / RATE as f32).exp()),
            release: (-9.0 / (release * RATE as f32)).exp(),
            releasing: false,
            room: tone.map_or(0.025, |t| t.room),
        }
    }
    fn next(&mut self) -> Option<[f32; 2]> {
        if self.wait > 0 {
            self.wait -= 1;
            return Some([0.0; 2]);
        }
        if self.envelope < 0.0001 {
            return None;
        }
        let value = if self.index < TONES {
            let tone = &bank().tones[self.index];
            let body = read(&tone.body, self.phase);
            let bright = read(&tone.bright, self.phase);
            self.phase += self.step;
            if self.phase >= SIZE as f64 {
                self.phase -= SIZE as f64;
            }
            body + (bright - body) * self.colour
        } else {
            *bank().drums[self.index - TONES].get(self.age as usize)?
        };
        let attack = (self.age as f32 / self.attack).min(1.0);
        let value = value * self.envelope * attack;
        self.age += 1;
        self.colour *= self.colour_decay;
        if self.age >= self.hold {
            self.releasing = true;
        }
        self.envelope *= if self.releasing {
            self.release
        } else {
            self.decay
        };
        Some(self.gain.map(|g| value * g))
    }
}
/// A fixed-capacity pool; note admission and rendering never allocate.
pub(super) struct Synth {
    voices: Vec<Voice>,
}
impl Default for Synth {
    fn default() -> Self {
        prepare();
        Self {
            voices: Vec::with_capacity(VOICES),
        }
    }
}
impl Synth {
    pub(super) fn note(
        &mut self,
        instrument: Instrument,
        pitch: u8,
        seconds: f32,
        gain: [f32; 2],
        touch: Touch,
    ) {
        if self.voices.len() < VOICES {
            self.voices
                .push(Voice::new(instrument, pitch, seconds, gain, touch));
        }
    }
    pub(super) fn release(&mut self) {
        for voice in &mut self.voices {
            voice.releasing = true;
        }
    }
    pub(super) fn frame(&mut self, out: &mut [f32; 2], send: &mut f32) {
        self.voices.retain_mut(|voice| {
            let Some(note) = voice.next() else {
                return false;
            };
            for i in 0..2 {
                out[i] += note[i];
            }
            *send += (note[0] + note[1]) * voice.room;
            true
        });
    }
    pub(super) fn render(&mut self, out: &mut [[f32; 2]], send: &mut [f32]) {
        self.voices.retain_mut(|voice| {
            for (frame, room) in out.iter_mut().zip(send.iter_mut()) {
                let Some(note) = voice.next() else {
                    return false;
                };
                for i in 0..2 {
                    frame[i] += note[i];
                }
                *room += (note[0] + note[1]) * voice.room;
            }
            true
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tables_are_periodic_dc_free_and_band_limited_at_the_highest_score_pitch() {
        for tone in &bank().tones {
            for table in [&tone.bright, &tone.body] {
                assert!((table[0] - table[SIZE]).abs() < f32::EPSILON);
                assert!(table.iter().all(|v| v.is_finite() && v.abs() <= 0.71));
                assert!(table[..SIZE].iter().sum::<f32>().abs() < 0.001);
                // DFT bins above the authored partials must contain no discontinuity energy.
                for bin in [13, 19, 63, 511] {
                    let mut re = 0.0_f64;
                    let mut im = 0.0_f64;
                    for (i, value) in table[..SIZE].iter().enumerate() {
                        let angle = std::f64::consts::TAU * f64::from(bin) * i as f64 / SIZE as f64;
                        re += f64::from(*value) * angle.cos();
                        im += f64::from(*value) * angle.sin();
                    }
                    assert!((re.hypot(im) / SIZE as f64) < 1e-7);
                }
            }
        }
        let frequency = bank().steps[88] * f64::from(RATE) / SIZE as f64;
        assert!(frequency * 12.0 < 20_000.0);
        assert!((bank().steps[69] * f64::from(RATE) / SIZE as f64 - 440.0).abs() < 1e-9);
    }
    #[test]
    fn pool_never_grows_and_all_voices_end_after_release() {
        let mut synth = Synth::default();
        for _ in 0..VOICES * 2 {
            synth.note(Instrument::Pad, 60, 10.0, [0.1; 2], Touch::at(0.0));
        }
        assert_eq!(synth.voices.len(), VOICES);
        assert_eq!(synth.voices.capacity(), VOICES);
        let mut out = [[0.0; 2]; 256];
        let mut feed = [0.0; 256];
        synth.render(&mut out, &mut feed);
        synth.release();
        for _ in 0..200 {
            out.fill([0.0; 2]);
            feed.fill(0.0);
            synth.render(&mut out, &mut feed);
        }
        assert!(synth.voices.is_empty());
    }
}
