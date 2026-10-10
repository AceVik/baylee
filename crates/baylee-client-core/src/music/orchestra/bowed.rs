//! Original harmonic excitation with instrument body resonances. These are
//! explicitly models, not recordings. Every partial stays below 18 kHz at
//! the voice's pitch (also allowing for detune/vibrato) at native 48 kHz.
use super::{Instrument, RATE, Touch};
use std::{f32::consts::TAU, sync::OnceLock};

const TABLE: usize = 2048;
const LOW: u8 = 28;
const HIGH: u8 = 88;
const PITCHES: usize = (HIGH - LOW + 1) as usize;
type Wave = [f32; TABLE + 1];

fn family(instrument: Instrument) -> usize {
    match instrument {
        Instrument::Violin | Instrument::ViolinShort => 0,
        Instrument::Viola | Instrument::ViolaShort => 1,
        Instrument::Cello | Instrument::CelloShort => 2,
        Instrument::Bass => 3,
        _ => 4,
    }
}

/// Evaluate a broad body resonance without baking a recorded instrument in.
fn resonance(frequency: f32, centre: f32, width: f32) -> f32 {
    1.0 / (1.0 + ((frequency - centre) / width).powi(2))
}

pub(super) fn tables() -> &'static [Box<Wave>] {
    static TABLES: OnceLock<Vec<Box<Wave>>> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut waves = Vec::with_capacity(PITCHES * 5);
        for kind in 0..5 {
            let (body, brilliance) = [
                (490.0, 2400.0),
                (380.0, 1800.0),
                (210.0, 1150.0),
                (105.0, 750.0),
                (620.0, 1600.0),
            ][kind];
            for pitch in LOW..=HIGH {
                let frequency = 440.0 * 2.0_f32.powf((f32::from(pitch) - 69.0) / 12.0);
                let mut wave = Box::new([0.0; TABLE + 1]);
                for harmonic in 1..=48 {
                    let overtone = frequency * harmonic as f32;
                    if overtone > 18_000.0 {
                        break;
                    }
                    let body_gain = 0.34
                        + 1.25 * resonance(overtone, body, body * 0.65)
                        + 0.75 * resonance(overtone, brilliance, brilliance * 0.38);
                    let rolloff = 1.0 / (1.0 + (overtone / 4400.0).powi(4));
                    let amplitude = body_gain * rolloff
                        / (harmonic as f32).powf(if kind == 4 { 1.16 } else { 1.28 });
                    // Alternating excitation phase, then the resonant body. No saw
                    // discontinuity: the actual waveform contains only these partials.
                    let phase = if kind == 4 { 0.17 } else { -0.12 } * harmonic as f32;
                    for (i, sample) in wave[..TABLE].iter_mut().enumerate() {
                        *sample += amplitude
                            * (TAU * harmonic as f32 * i as f32 / TABLE as f32 + phase).sin();
                    }
                }
                let peak = wave[..TABLE]
                    .iter()
                    .copied()
                    .map(f32::abs)
                    .fold(0.0, f32::max);
                for sample in &mut wave[..TABLE] {
                    *sample *= 0.62 / peak.max(0.001);
                }
                wave[TABLE] = wave[0];
                waves.push(wave);
            }
        }
        waves
    })
}

fn read(wave: &Wave, phase: f32) -> f32 {
    let at = phase as usize;
    wave[at] + (wave[at + 1] - wave[at]) * (phase - at as f32)
}

pub(super) struct Voice {
    wave: &'static Wave,
    phases: [f32; 3],
    step: f32,
    vibrato: f32,
    vibrato_step: f32,
    age: u32,
    wait: u32,
    hold: u32,
    attack: f32,
    release: f32,
    envelope: f32,
    released: bool,
    gain: [f32; 2],
    low: f32,
    tone: f32,
}
impl Voice {
    #[cfg(test)]
    pub(super) const fn level(&self) -> f32 {
        self.envelope
    }
    pub(super) fn new(
        instrument: Instrument,
        pitch: u8,
        seconds: f32,
        gain: [f32; 2],
        touch: Touch,
    ) -> Self {
        let kind = family(instrument);
        let pitch = pitch.clamp(LOW, HIGH);
        let frequency = 440.0 * 2.0_f32.powf((f32::from(pitch) - 69.0) / 12.0);
        let attack =
            touch
                .attack
                .unwrap_or(0.07)
                .max(if instrument.short() { 0.008 } else { 0.055 });
        let release = touch.release.unwrap_or(0.24).max(0.025);
        Self {
            wave: &tables()[kind * PITCHES + usize::from(pitch - LOW)],
            phases: [0.0, TABLE as f32 * 0.13, TABLE as f32 * 0.29],
            step: frequency * TABLE as f32 / RATE as f32,
            vibrato: f32::from(pitch) * 0.07,
            vibrato_step: (4.8 + kind as f32 * 0.19) / RATE as f32,
            age: 0,
            wait: (touch.late.max(0.0) * RATE as f32) as u32,
            hold: (seconds.max(0.0) * RATE as f32) as u32,
            attack: 1.0 / (attack * RATE as f32),
            release: 1.0 / (release * RATE as f32),
            envelope: 0.0,
            released: false,
            gain,
            low: 0.0,
            tone: if kind == 4 { 0.19 } else { 0.29 },
        }
    }
    pub(super) fn release(&mut self) {
        if !self.released {
            self.released = true;
            self.release = self.envelope / (RATE as f32 * 0.45);
        }
    }
    pub(super) fn next(&mut self) -> Option<[f32; 2]> {
        if self.wait > 0 {
            self.wait -= 1;
            return Some([0.0; 2]);
        }
        if self.age >= self.hold {
            self.released = true;
        }
        if self.released {
            self.envelope = (self.envelope - self.release).max(0.0);
            if self.envelope <= 0.000_001 {
                return None;
            }
        } else {
            self.envelope = (self.envelope + self.attack).min(1.0);
        }
        // Smooth polynomial LFO: no transcendental functions in the callback.
        self.vibrato += self.vibrato_step;
        self.vibrato -= self.vibrato.floor();
        let triangle = 1.0 - (self.vibrato * 4.0 - 2.0).abs();
        let modulation = triangle * (1.5 - 0.5 * triangle * triangle);
        let depth = (self.age as f32 / (RATE as f32 * 0.6)).min(1.0);
        let mut wave = 0.0;
        for (i, phase) in self.phases.iter_mut().enumerate() {
            wave += read(self.wave, *phase) * [0.50, 0.27, 0.23][i];
            *phase += self.step * ([1.0, 0.9986, 1.0014][i] + modulation * depth * 0.0012);
            if *phase >= TABLE as f32 {
                *phase -= TABLE as f32;
            }
        }
        self.low += (wave - self.low) * self.tone;
        self.age += 1;
        // Smooth attack and release derivatives at both ends.
        let envelope = self.envelope * self.envelope * (3.0 - 2.0 * self.envelope);
        Some(self.gain.map(|gain| gain * self.low * envelope))
    }
}
