//! Original, band-limited additive/orchestral patches and synthesized percussion, native 48 kHz.
//! Includes warm French horn ensemble swells, lush bowed string pads, crystalline celesta,
//! and pitched orchestral timpani. Tables and pitch increments are prepared outside the
//! callback. No samples, presets, loops or melodies from a third-party synthesizer are embedded.
use super::{Instrument, RATE, Touch};
use std::sync::OnceLock;
const SIZE: usize = 2048;
const TONES: usize = 14;
const VOICES: usize = 80;
/// Symmetric +/-2.00-cent stereo ensemble detune (`2^(+/-2/1200)`), keeping chord center at 0.0c.
const DETUNE_UP: f64 = 1.001_155_96;
const DETUNE_DOWN: f64 = 0.998_845_38;

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
    // Highest memorial pitch is 90 (1480 Hz): partial 12 stays below 18 kHz.
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
        AnalogLead, Bell, Celesta, ChipLead, Clav, ElectricPiano, Flute, FrenchHorn, Guitar,
        Marimba, Pad, Piano, StringPad, SynthBass,
    };
    let (partials, attack, decay, colour, room) = match instrument {
        Piano => (
            [
                1.0, 0.52, 0.26, 0.15, 0.08, 0.045, 0.025, 0.015, 0.008, 0.0, 0.0, 0.0,
            ],
            0.004,
            0.58,
            3.6,
            0.12,
        ),
        Clav => (
            [
                1.0, 0.62, 0.44, 0.30, 0.20, 0.13, 0.08, 0.05, 0.03, 0.018, 0.01, 0.005,
            ],
            0.002,
            1.3,
            2.8,
            0.08,
        ),
        Flute => (
            [
                1.0, 0.08, 0.18, 0.03, 0.035, 0.01, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
            0.024,
            0.04,
            1.9,
            0.12,
        ),
        Guitar => (
            [
                1.0, 0.40, 0.26, 0.13, 0.06, 0.04, 0.018, 0.01, 0.0, 0.0, 0.0, 0.0,
            ],
            0.003,
            1.1,
            5.8,
            0.07,
        ),
        ElectricPiano => (
            [
                1.0, 0.26, 0.32, 0.12, 0.06, 0.035, 0.0, 0.018, 0.0, 0.0, 0.0, 0.0,
            ],
            0.006,
            0.72,
            4.8,
            0.09,
        ),
        AnalogLead => (
            [
                1.0, 0.44, 0.24, 0.13, 0.075, 0.045, 0.026, 0.016, 0.01, 0.006, 0.0, 0.0,
            ],
            0.012,
            0.09,
            6.5,
            0.07,
        ),
        ChipLead => (
            [
                1.0, 0.0, 0.31, 0.0, 0.16, 0.0, 0.09, 0.0, 0.055, 0.0, 0.032, 0.0,
            ],
            0.003,
            0.14,
            1.1,
            0.015,
        ),
        Marimba => (
            [
                1.0, 0.0, 0.0, 0.44, 0.0, 0.0, 0.0, 0.08, 0.0, 0.04, 0.0, 0.0,
            ],
            0.002,
            2.8,
            13.0,
            0.12,
        ),
        Bell => (
            [
                1.0, 0.30, 0.40, 0.12, 0.08, 0.11, 0.0, 0.04, 0.0, 0.0, 0.0, 0.0,
            ],
            0.004,
            0.72,
            2.0,
            0.24,
        ),
        Pad => (
            [
                1.0, 0.22, 0.20, 0.09, 0.07, 0.035, 0.0, 0.015, 0.0, 0.0, 0.0, 0.0,
            ],
            0.20,
            0.012,
            0.9,
            0.28,
        ),
        SynthBass => (
            [
                1.0, 0.28, 0.14, 0.065, 0.03, 0.012, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
            0.009,
            0.35,
            6.8,
            0.015,
        ),
        FrenchHorn => (
            [
                1.0, 0.82, 0.62, 0.43, 0.27, 0.16, 0.085, 0.042, 0.018, 0.0, 0.0, 0.0,
            ],
            0.065,
            0.032,
            1.35,
            0.26,
        ),
        StringPad => (
            [
                1.0, 0.56, 0.37, 0.24, 0.16, 0.105, 0.068, 0.042, 0.025, 0.014, 0.007, 0.0,
            ],
            0.14,
            0.010,
            0.78,
            0.30,
        ),
        Celesta => (
            [
                1.0, 0.22, 0.10, 0.26, 0.0, 0.10, 0.0, 0.035, 0.0, 0.0, 0.0, 0.0,
            ],
            0.003,
            1.45,
            3.9,
            0.26,
        ),
        _ => unreachable!("melodic synthetic palette"),
    };
    let mut body = partials;
    for (i, partial) in body.iter_mut().enumerate() {
        let rolloff = if instrument == ChipLead {
            1.0 + 0.35 * i as f32
        } else {
            (i + 1) as f32
        };
        *partial /= rolloff;
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
            Instrument::FrenchHorn,
            Instrument::StringPad,
            Instrument::Celesta,
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
#[derive(Clone, Copy)]
enum Source {
    Tone(&'static Timbre),
    Drum(&'static [f32]),
    Kettle { f0: f32 },
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Articulation {
    Direct,
    BreathVibrato,
    Ensemble,
    BowedEnsemble,
}
struct Voice {
    source: Source,
    articulation: Articulation,
    phase_up: f64,
    phase_down: f64,
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
        let clamped = pitch.clamp(28, 88);
        let index = instrument as usize - Instrument::Piano as usize;
        let tone = bank().tones.get(index);
        let release = if matches!(
            instrument,
            Instrument::Pad | Instrument::StringPad | Instrument::FrenchHorn | Instrument::Celesta
        ) {
            0.82
        } else if instrument == Instrument::Timpani {
            0.72
        } else {
            0.46
        };
        let cent_ratio = if touch.cents == 0.0 {
            1.0
        } else {
            2.0_f64.powf(f64::from(touch.cents) / 1200.0)
        };
        let source = if instrument == Instrument::Timpani {
            let f0 = 440.0
                * 2.0_f32
                    .powf((f32::from(pitch.clamp(28, 64)) - 69.0 + touch.cents / 100.0) / 12.0);
            Source::Kettle { f0 }
        } else {
            tone.map_or_else(|| Source::Drum(&bank().drums[index - TONES]), Source::Tone)
        };
        // Gently soften initial upper-partial brightness above F#4 to avoid 3-5 kHz roughness.
        let reg_colour =
            (1.0 - f32::from(clamped.saturating_sub(66)) / 38.0 * 0.40).clamp(0.56, 1.0);
        let articulation = match instrument {
            Instrument::StringPad => Articulation::BowedEnsemble,
            Instrument::Pad | Instrument::FrenchHorn => Articulation::Ensemble,
            Instrument::Flute => Articulation::BreathVibrato,
            _ => Articulation::Direct,
        };
        Self {
            source,
            articulation,
            phase_up: SIZE as f64 * 0.125,
            phase_down: SIZE as f64 * 0.875,
            phase: 0.0,
            step: bank().steps[usize::from(clamped)] * cent_ratio,
            age: 0,
            wait: (touch.late * RATE as f32) as u32,
            hold: (seconds * RATE as f32) as u32,
            gain,
            attack: if instrument == Instrument::Timpani {
                0.003 * RATE as f32
            } else {
                tone.map_or(1.0, |t| t.attack * RATE as f32)
            },
            envelope: 1.0,
            decay: if instrument == Instrument::Timpani {
                (-2.5 / RATE as f32).exp()
            } else {
                tone.map_or(1.0, |t| (-t.decay / RATE as f32).exp())
            },
            colour: reg_colour,
            colour_decay: tone.map_or(1.0, |t| (-t.colour_decay / RATE as f32).exp()),
            release: (-9.4 / (release * RATE as f32)).exp(),
            releasing: false,
            room: if instrument == Instrument::Timpani {
                0.22
            } else {
                tone.map_or(0.025, |t| t.room)
            },
        }
    }
    fn release(&mut self) {
        if !self.releasing {
            // Freeze the attained attack level: an interrupted pad cannot bloom later.
            self.envelope *= (self.age as f32 / self.attack).min(1.0);
            self.attack = 1.0;
            self.releasing = true;
        }
    }
    fn step_tone(&mut self, tone: &'static Timbre) -> [f32; 2] {
        let t = self.age as f32 / RATE as f32;
        let has_vibrato = matches!(
            self.articulation,
            Articulation::BreathVibrato | Articulation::BowedEnsemble
        );
        let has_chorus = matches!(
            self.articulation,
            Articulation::Ensemble | Articulation::BowedEnsemble
        );
        let vib_env = if has_vibrato {
            ((t - 0.22) * 2.8).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let lfo = self.age as f32 * (std::f32::consts::TAU * 5.1 / RATE as f32);
        let step = if vib_env > 0.0 {
            self.step * (1.0 + 0.000_82 * f64::from(vib_env * lfo.sin()))
        } else {
            self.step
        };
        let body = read(&tone.body, self.phase);
        let bright = read(&tone.bright, self.phase);
        self.phase += step;
        if self.phase >= SIZE as f64 {
            self.phase -= SIZE as f64;
        }
        let note = body + (bright - body) * self.colour;
        let stereo = if has_chorus {
            // Symmetric +2c / -2c stereo ensemble around 0c locks chord centers without beating.
            let up = read(&tone.body, self.phase_up);
            let down = read(&tone.body, self.phase_down);
            self.phase_up += step * DETUNE_UP;
            if self.phase_up >= SIZE as f64 {
                self.phase_up -= SIZE as f64;
            }
            self.phase_down += step * DETUNE_DOWN;
            if self.phase_down >= SIZE as f64 {
                self.phase_down -= SIZE as f64;
            }
            [
                note * 0.68 + up * 0.21 + down * 0.11,
                note * 0.68 + down * 0.21 + up * 0.11,
            ]
        } else {
            [note, note]
        };
        if self.articulation == Articulation::BowedEnsemble && vib_env > 0.0 {
            let expr = 1.0 - 0.032 * vib_env * (1.0 - lfo.cos());
            [stereo[0] * expr, stereo[1] * expr]
        } else {
            stereo
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
        let stereo = match self.source {
            Source::Tone(tone) => self.step_tone(tone),
            Source::Drum(data) => {
                let v = *data.get(self.age as usize)?;
                [v, v]
            }
            Source::Kettle { f0 } => {
                let t = self.age as f32 / RATE as f32;
                // Circular kettle membrane: brief impact tension bend + 1.504x / 2.0x modes.
                let bend = 1.0 + 0.065 * (-t * 34.0).exp();
                let dp = std::f64::consts::TAU * f64::from(f0 * bend) / f64::from(RATE);
                self.phase = (self.phase + dp).rem_euclid(std::f64::consts::TAU);
                self.phase_up = (self.phase_up + dp * 1.504).rem_euclid(std::f64::consts::TAU);
                let head = (self.phase.sin() as f32) * 0.74
                    + (self.phase_up.sin() as f32) * 0.20 * (-t * 6.0).exp()
                    + ((self.phase * 2.0).sin() as f32) * 0.08 * (-t * 9.0).exp();
                let mallet =
                    (std::f32::consts::TAU * f0 * 3.0 * t).sin() * (-t * 95.0).exp() * 0.16;
                let v = (head + mallet) * 0.62;
                [v, v]
            }
        };
        let attack = (self.age as f32 / self.attack).min(1.0);
        let amp = self.envelope * attack;
        self.age += 1;
        self.colour *= self.colour_decay;
        if self.age >= self.hold {
            self.release();
        }
        self.envelope *= if self.releasing {
            self.release
        } else {
            self.decay
        };
        Some([
            stereo[0] * amp * self.gain[0],
            stereo[1] * amp * self.gain[1],
        ])
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
            voice.release();
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

#[cfg(test)]
#[test]
fn interrupted_attacks_and_repeated_releases_never_revive() {
    for delay in [0, 17, 1000, 12000] {
        let mut voice = Voice::new(Instrument::Pad, 60, 2.0, [1.0; 2], Touch::at(0.0));
        for _ in 0..delay {
            voice.next();
        }
        let attained = voice.envelope * (voice.age as f32 / voice.attack).min(1.0);
        voice.release();
        assert!((voice.envelope - attained).abs() < 1e-7);
        for _ in 0..2000 {
            let before = voice.envelope;
            voice.release();
            voice.next();
            assert!(voice.envelope <= before);
        }
    }
}
