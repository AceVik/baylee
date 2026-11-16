//! Allocated-once physical-modelled plucked strings: Renaissance lute/lyre,
//! Acoustic Tavern Guitar, and Concert Grand Harp with Sitka spruce soundboard
//! resonators and sympathetic open-string bloom. Zero render-time heap allocation.
use crate::music::RATE;

/// Longest delay line a string holds: down to ~24 Hz at 48 kHz.
const STRING: usize = 2048;
/// Polyphony for the modelled lute/lyre.
pub(super) const STRINGS: usize = 24;
/// Polyphony for the physical-modelled Acoustic Tavern Guitar.
pub(super) const GUITAR_STRINGS: usize = 16;
/// Polyphony for the physical-modelled Concert Grand Harp.
pub(super) const HARP_STRINGS: usize = 24;

/// Exact first-order all-pass coefficient yielding phase delay `frac_delay` at `omega0`:
/// solves `arg((c + e^{-j w0}) / (1 + c e^{-j w0})) = -w0 * frac_delay` in closed form.
/// Keeping `frac_delay` in `[0.5, 1.5)` bounds `|c| <= 0.34`, avoiding Nyquist pole ringing.
#[inline]
fn exact_allpass_coeff(omega0: f64, frac_delay: f64) -> f32 {
    let half_w = 0.5 * omega0;
    let denom = (half_w * (1.0 + frac_delay)).sin();
    if denom.abs() < 1e-12 {
        ((1.0 - frac_delay) / (1.0 + frac_delay)) as f32
    } else {
        ((half_w * (1.0 - frac_delay)).sin() / denom).clamp(-0.95, 0.95) as f32
    }
}

#[inline]
fn pitch_hz(pitch: u8, cents: f32) -> f64 {
    440.0 * 2.0_f64.powf((f64::from(pitch) - 69.0 + f64::from(cents) / 100.0) / 12.0)
}

/// A plucked string synthesised by Karplus–Strong: a delay line of one
/// period, filled with a burst of filtered noise and `1/n^p` harmonics, fed back
/// through a two-tap average and exact-phase-compensated all-pass filter.
/// Its buffer is allocated once, with the orchestra.
pub(super) struct Lute {
    line: Box<[f32; STRING]>,
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

    /// Plucks it: a DC-free burst of warm filtered noise and `1/n^p` harmonics from `seed`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn pluck(
        &mut self,
        pitch: u8,
        seconds: f32,
        gain: [f32; 2],
        bright: f32,
        decay: f32,
        seed: u32,
        cents: f32,
    ) {
        self.decay = decay;
        let frequency = pitch_hz(pitch.clamp(28, 88), cents);
        let target_delay = (f64::from(RATE) / frequency).clamp(2.5, (STRING - 2) as f64);
        // Two-tap average has exact linear-phase delay `period - 0.5`; choosing
        // `period = floor(target_delay)` keeps `frac_delay` in `[0.5, 1.5)`.
        self.period = (target_delay.floor() as usize).max(2);
        let frac_delay = target_delay - (self.period as f64 - 0.5);
        let omega0 = std::f64::consts::TAU / target_delay;
        self.allpass = exact_allpass_coeff(omega0, frac_delay);

        let mut state = seed.wrapping_mul(0x9E37_79B9) | 1;
        let cutoff = (1.0 - bright).clamp(0.08, 0.45);
        let mut lp1 = 0.0_f32;
        let mut lp2 = 0.0_f32;
        let mut mean = 0.0_f32;
        for (i, slot) in self.line[..self.period].iter_mut().enumerate() {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let noise = (state as f32 / u32::MAX as f32) * 2.0 - 1.0;
            lp1 += (noise - lp1) * cutoff;
            lp2 += (lp1 - lp2) * cutoff;
            let phase = std::f32::consts::TAU * i as f32 / self.period as f32;
            // Smooth 1/n^1.75 harmonic series avoids high-frequency metallic aliasing.
            let harmonics = phase.sin() * 0.30
                + (2.0 * phase).sin() * 0.090
                + (3.0 * phase).sin() * 0.044
                + (4.0 * phase).sin() * 0.026;
            let sample = lp2 * 0.24 + harmonics;
            *slot = sample;
            mean += sample;
        }
        mean /= self.period as f32;
        for slot in &mut self.line[..self.period] {
            *slot -= mean;
        }
        let last = self.line[self.period - 1];
        self.allpass_in = last;
        self.allpass_out = last;
        self.at = 0;
        self.age = 0;
        self.hold = (seconds * RATE as f32) as u32;
        self.release = (0.36 * RATE as f32) as u32;
        self.gain = gain;
        self.sounding = true;
    }

    /// Smoothly release a ringing tail without altering its current envelope or phase.
    pub(super) fn release(&mut self) {
        let fade = (RATE as f32 * 0.78) as u32;
        let remaining = self.hold.saturating_sub(self.age) + self.release;
        if remaining > fade && self.age < self.hold {
            self.hold = self.age;
            self.release = fade;
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
        let lin = (1.0 - self.age.saturating_sub(self.hold) as f32 / self.release as f32).max(0.0);
        let release = lin * lin * (3.0 - 2.0 * lin);
        self.age += 1;
        let attack = (self.age as f32 / (RATE as f32 * 0.004)).min(1.0);
        Some(self.gain.map(|gain| gain * out * release * attack))
    }
}

/// Zero-delay-feedback unity-peak bandpass resonator (trapezoidal SVF) for
/// guitar soundboard modes and sympathetic open-string coupling.
#[derive(Clone, Copy)]
struct Resonator {
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
    ic1eq: f32,
    ic2eq: f32,
}

impl Resonator {
    fn new(freq_hz: f32, q: f32) -> Self {
        let g = (std::f32::consts::PI * freq_hz / RATE as f32).tan();
        let k = 1.0 / q;
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        Self {
            a1,
            a2,
            a3,
            k,
            ic1eq: 0.0,
            ic2eq: 0.0,
        }
    }

    #[inline]
    fn step(&mut self, v0: f32) -> f32 {
        let v3 = v0 - self.ic2eq;
        let v1 = self.a1 * self.ic1eq + self.a2 * v3;
        let v2 = self.ic2eq + self.a2 * self.ic1eq + self.a3 * v3;
        self.ic1eq = 2.0 * v1 - self.ic1eq;
        self.ic2eq = 2.0 * v2 - self.ic2eq;
        self.k * v1
    }
}

/// One nylon/gut string of the Acoustic Tavern Guitar: symmetric 3-tap
/// linear-phase bridge loss filter + fractional Thiran dispersion/tuning
/// all-pass, excited by thumb-flesh bass or finger-plucked treble profiles.
struct GuitarString {
    line: Box<[f32; STRING]>,
    period: usize,
    allpass: f32,
    decay: f32,
    damp: f32,
    allpass_in: f32,
    allpass_out: f32,
    at: usize,
    age: u32,
    wait: u32,
    hold: u32,
    release: u32,
    gain: [f32; 2],
    body_send: f32,
    sounding: bool,
}

impl GuitarString {
    fn new() -> Self {
        Self {
            line: Box::new([0.0; STRING]),
            period: 4,
            allpass: 0.0,
            decay: 0.9985,
            damp: 0.08,
            allpass_in: 0.0,
            allpass_out: 0.0,
            at: 0,
            age: 0,
            wait: 0,
            hold: 0,
            release: 1,
            gain: [0.0; 2],
            body_send: 0.35,
            sounding: false,
        }
    }

    fn pluck(&mut self, pitch: u8, seconds: f32, gain: [f32; 2], late: f32, cents: f32, seed: u32) {
        let pitch = pitch.clamp(33, 88);
        let reg = ((f32::from(pitch) - 38.0) / 44.0).clamp(0.0, 1.0);
        let frequency = pitch_hz(pitch, cents);
        // Symmetric 3-tap FIR centered at s1 has exact linear-phase delay `period - 1 = int_delay`.
        let target_delay = (f64::from(RATE) / frequency).clamp(4.0, (STRING - 2) as f64);
        let int_delay = (target_delay - 0.5).floor().max(3.0);
        self.period = (int_delay as usize) + 1;
        let rest = target_delay - int_delay;
        let omega0 = std::f64::consts::TAU / target_delay;
        self.allpass = exact_allpass_coeff(omega0, rest);

        // Wound bass strings sustain warmly with gentle damping; gut/nylon trebles sing clearly.
        self.decay = 0.9990 - 0.0014 * reg;
        self.damp = 0.052 + 0.068 * (1.0 - reg);
        self.body_send = 0.42 - 0.16 * reg;

        // Thumb plucks near the soundhole (~0.24L); right-hand fingers pluck sweeter (~0.16L).
        let pick_pos = if pitch < 53 {
            0.24
        } else if pitch < 64 {
            0.19
        } else {
            0.155
        };
        let apex = ((self.period as f32 * pick_pos).round() as usize).clamp(1, self.period - 1);
        // Two-pole flesh/nail filter: soft warm thumb pad on bass, articulate finger release on treble.
        let cutoff = 0.09 + 0.26 * reg;
        let mut state =
            seed.wrapping_mul(0x85EB_CA6B) ^ u32::from(pitch).wrapping_mul(0xC2B2_AE35) | 1;
        let mut lp1 = 0.0_f32;
        let mut lp2 = 0.0_f32;
        let mut mean = 0.0_f32;
        for (i, slot) in self.line[..self.period].iter_mut().enumerate() {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let noise = (state as f32 / u32::MAX as f32) * 2.0 - 1.0;
            lp1 += (noise - lp1) * cutoff;
            lp2 += (lp1 - lp2) * cutoff;
            // Plucked string displacement triangle + smooth 1/n^1.8 soundhole harmonics.
            let tri = if i < apex {
                i as f32 / apex as f32
            } else {
                (self.period - i) as f32 / (self.period - apex) as f32
            };
            let phase = std::f32::consts::TAU * i as f32 / self.period as f32;
            let fundamental = phase.sin() * (0.34 - 0.07 * reg)
                + (2.0 * phase).sin() * 0.10
                + (3.0 * phase).sin() * 0.045;
            let transient = lp2 * (0.19 + 0.14 * reg);
            let sample = (tri - 0.5) * 0.40 + fundamental + transient;
            *slot = sample;
            mean += sample;
        }
        mean /= self.period as f32;
        for slot in &mut self.line[..self.period] {
            *slot -= mean;
        }
        let last = self.line[self.period - 1];
        self.allpass_in = last;
        self.allpass_out = last;

        self.at = 0;
        self.age = 0;
        self.wait = (late.max(0.0) * RATE as f32) as u32;
        self.hold = (seconds * RATE as f32) as u32;
        self.release = (0.48 * RATE as f32) as u32;
        // Subtle close-mic stereo spread: bass strings slightly left, treble slightly right.
        let spread = (reg - 0.45) * 0.14;
        self.gain = [
            gain[0] * (1.0 - spread.max(0.0)),
            gain[1] * (1.0 + spread.min(0.0)),
        ];
        self.sounding = true;
    }

    fn release(&mut self) {
        let fade = (RATE as f32 * 0.84) as u32;
        let remaining = self.hold.saturating_sub(self.age) + self.release;
        if remaining > fade && self.age < self.hold {
            self.hold = self.age;
            self.release = fade;
        }
    }

    /// Returns stereo direct string frame and mono bridge force sent to the guitar body.
    #[inline]
    fn next(&mut self) -> Option<([f32; 2], f32)> {
        if !self.sounding {
            return None;
        }
        if self.wait > 0 {
            self.wait -= 1;
            return Some(([0.0; 2], 0.0));
        }
        if self.age >= self.hold + self.release {
            self.sounding = false;
            return None;
        }
        let s0 = self.line[self.at];
        let i1 = if self.at + 1 >= self.period {
            self.at + 1 - self.period
        } else {
            self.at + 1
        };
        let i2 = if i1 + 1 >= self.period {
            i1 + 1 - self.period
        } else {
            i1 + 1
        };
        let s1 = self.line[i1];
        let s2 = self.line[i2];
        let filtered = self.decay * ((1.0 - 2.0 * self.damp) * s1 + self.damp * (s0 + s2));
        let tuned = self.allpass * (filtered - self.allpass_out) + self.allpass_in;
        self.allpass_in = filtered;
        self.allpass_out = tuned;
        self.line[self.at] = tuned;
        self.at = i1;

        let lin = (1.0 - self.age.saturating_sub(self.hold) as f32 / self.release as f32).max(0.0);
        let release = lin * lin * (3.0 - 2.0 * lin);
        self.age += 1;
        // Soft 3.5 ms flesh-release ramp avoids click on voice steal while preserving transient.
        let attack = (self.age as f32 / (RATE as f32 * 0.0035)).min(1.0);
        let env = release * attack;
        let bridge = s1 * env;
        Some((
            [bridge * self.gain[0], bridge * self.gain[1]],
            bridge * (self.gain[0] + self.gain[1]) * 0.5 * self.body_send,
        ))
    }
}

/// Complete polyphonic Acoustic Tavern Guitar with coupled Helmholtz air (~98 Hz),
/// spruce top-plate (~205 Hz), upper dipole (~395 Hz) soundboard resonators, and
/// subtle D3/A3 sympathetic open-string resonance.
pub(super) struct TavernGuitar {
    strings: [GuitarString; GUITAR_STRINGS],
    air_98: Resonator,
    top_205: Resonator,
    wood_395: Resonator,
    symp_d3: Resonator,
    symp_a3: Resonator,
    plucks: u32,
    tail_frames: u32,
}

impl TavernGuitar {
    pub(super) fn new() -> Self {
        Self {
            strings: std::array::from_fn(|_| GuitarString::new()),
            air_98: Resonator::new(98.0, 6.8),
            top_205: Resonator::new(205.0, 7.5),
            wood_395: Resonator::new(395.0, 5.8),
            symp_d3: Resonator::new(146.832, 34.0),
            symp_a3: Resonator::new(220.000, 34.0),
            plucks: 0,
            tail_frames: 0,
        }
    }

    pub(super) fn pluck(&mut self, pitch: u8, seconds: f32, gain: [f32; 2], late: f32, cents: f32) {
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
        self.strings[slot].pluck(pitch, seconds, gain, late, cents, self.plucks);
        self.tail_frames = RATE / 2;
    }

    pub(super) fn release(&mut self) {
        for string in &mut self.strings {
            string.release();
        }
    }

    #[inline]
    fn step_body(&mut self, bridge: f32) -> [f32; 2] {
        let air = self.air_98.step(bridge) * 0.30;
        let top = self.top_205.step(bridge) * 0.26;
        let wood = self.wood_395.step(bridge) * 0.20;
        let symp_d = self.symp_d3.step(bridge) * 0.038;
        let symp_a = self.symp_a3.step(bridge) * 0.032;
        [
            air + top * 1.06 + wood * 0.92 + symp_d,
            air + top * 0.94 + wood * 1.08 + symp_a,
        ]
    }

    #[inline]
    pub(super) fn frame(&mut self, dry: &mut [f32; 2], feed: &mut f32) {
        let mut any = false;
        let mut direct = [0.0_f32; 2];
        let mut bridge = 0.0_f32;
        for string in &mut self.strings {
            if let Some((pair, force)) = string.next() {
                any = true;
                direct[0] += pair[0];
                direct[1] += pair[1];
                bridge += force;
            }
        }
        if any {
            self.tail_frames = RATE / 2;
        } else if self.tail_frames == 0 {
            return;
        } else {
            self.tail_frames -= 1;
        }
        let body = self.step_body(bridge);
        let l = direct[0] + body[0];
        let r = direct[1] + body[1];
        dry[0] += l;
        dry[1] += r;
        *feed += (l + r) * 0.21;
    }

    pub(super) fn render(&mut self, out: &mut [[f32; 2]], feed: &mut [f32]) {
        // Identical per-frame summation order to `frame()` for bit-for-bit block/scalar parity.
        let any_sounding = self.strings.iter().any(|s| s.sounding);
        if !any_sounding && self.tail_frames == 0 {
            return;
        }
        for (frame, send) in out.iter_mut().zip(feed.iter_mut()) {
            self.frame(frame, send);
        }
    }
}

/// One gut/nylon string of the Concert Grand Harp: exact linear-phase 3-tap bridge
/// damping + closed-form fractional phase-delay Thiran all-pass (`< 0.15 cents`),
/// excited near the string's sweet center (`0.32..0.38L`) by a velvet finger-pad
/// profile with steep `1/n^2.4` upper-partial rolloff so highs shimmer gently
/// without metallic bite or whistle.
pub(super) struct HarpString {
    line: Box<[f32; STRING]>,
    period: usize,
    allpass: f32,
    decay: f32,
    damp: f32,
    allpass_in: f32,
    allpass_out: f32,
    at: usize,
    pub(super) age: u32,
    wait: u32,
    hold: u32,
    release: u32,
    gain: [f32; 2],
    body_send: f32,
    pub(super) sounding: bool,
}

impl HarpString {
    pub(super) fn new() -> Self {
        Self {
            line: Box::new([0.0; STRING]),
            period: 4,
            allpass: 0.0,
            decay: 0.9992,
            damp: 0.04,
            allpass_in: 0.0,
            allpass_out: 0.0,
            at: 0,
            age: 0,
            wait: 0,
            hold: 0,
            release: 1,
            gain: [0.0; 2],
            body_send: 0.34,
            sounding: false,
        }
    }

    pub(super) fn pluck(
        &mut self,
        pitch: u8,
        seconds: f32,
        gain: [f32; 2],
        late: f32,
        cents: f32,
        seed: u32,
    ) {
        let pitch = pitch.clamp(33, 88);
        let reg = ((f32::from(pitch) - 38.0) / 48.0).clamp(0.0, 1.0);
        let frequency = pitch_hz(pitch, cents);
        let target_delay = (f64::from(RATE) / frequency).clamp(4.0, (STRING - 2) as f64);
        let int_delay = (target_delay - 0.5).floor().max(3.0);
        self.period = (int_delay as usize) + 1;
        let rest = target_delay - int_delay;
        self.allpass = exact_allpass_coeff(std::f64::consts::TAU / target_delay, rest);

        // Long singing concert-harp sustain with velvet high-partial air absorption.
        self.decay = 0.99935 - 0.00080 * reg;
        self.damp = 0.030 + 0.046 * (1.0 - reg);
        self.body_send = 0.38 - 0.14 * reg;

        let apex =
            ((self.period as f32 * (0.38 - 0.06 * reg)).round() as usize).clamp(1, self.period - 1);
        let cutoff = 0.075 + 0.14 * reg;
        let mut state =
            seed.wrapping_mul(0x9E37_79B9) ^ u32::from(pitch).wrapping_mul(0x85EB_CA6B) | 1;
        let (mut lp1, mut lp2, mut lp3, mut mean) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
        for (i, slot) in self.line[..self.period].iter_mut().enumerate() {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let noise = (state as f32 / u32::MAX as f32) * 2.0 - 1.0;
            lp1 += (noise - lp1) * cutoff;
            lp2 += (lp1 - lp2) * cutoff;
            lp3 += (lp2 - lp3) * cutoff;
            let tri = if i < apex {
                i as f32 / apex as f32
            } else {
                (self.period - i) as f32 / (self.period - apex) as f32
            };
            // Rounded finger-flesh displacement arch + steep 1/n^2.4 partial series.
            let arch = tri * tri * (3.0 - 2.0 * tri) - 0.5;
            let p = std::f32::consts::TAU * i as f32 / self.period as f32;
            let harmonics = p.sin() * (0.42 - 0.05 * reg)
                + (2.0 * p).sin() * 0.080
                + (3.0 * p).sin() * 0.030
                + (4.0 * p).sin() * 0.015
                + (5.0 * p).sin() * 0.0088;
            let sample = arch * 0.28 + harmonics + lp3 * 0.085;
            *slot = sample;
            mean += sample;
        }
        mean /= self.period as f32;
        for slot in &mut self.line[..self.period] {
            *slot -= mean;
        }
        let last = self.line[self.period - 1];
        self.allpass_in = last;
        self.allpass_out = last;
        self.at = 0;
        self.age = 0;
        self.wait = (late.max(0.0) * RATE as f32) as u32;
        self.hold = (seconds * RATE as f32) as u32;
        self.release = (0.62 * RATE as f32) as u32;
        let spread = (reg - 0.45) * 0.18;
        self.gain = [
            gain[0] * (1.0 - spread.max(0.0)),
            gain[1] * (1.0 + spread.min(0.0)),
        ];
        self.sounding = true;
    }

    pub(super) fn release(&mut self) {
        let fade = (RATE as f32 * 0.90) as u32;
        let remaining = self.hold.saturating_sub(self.age) + self.release;
        if remaining > fade && self.age < self.hold {
            self.hold = self.age;
            self.release = fade;
        }
    }

    #[inline]
    pub(super) fn next(&mut self) -> Option<([f32; 2], f32)> {
        if !self.sounding {
            return None;
        }
        if self.wait > 0 {
            self.wait -= 1;
            return Some(([0.0; 2], 0.0));
        }
        if self.age >= self.hold + self.release {
            self.sounding = false;
            return None;
        }
        let s0 = self.line[self.at];
        let i1 = if self.at + 1 >= self.period {
            self.at + 1 - self.period
        } else {
            self.at + 1
        };
        let i2 = if i1 + 1 >= self.period {
            i1 + 1 - self.period
        } else {
            i1 + 1
        };
        let s1 = self.line[i1];
        let s2 = self.line[i2];
        let filtered = self.decay * ((1.0 - 2.0 * self.damp) * s1 + self.damp * (s0 + s2));
        let tuned = self.allpass * (filtered - self.allpass_out) + self.allpass_in;
        self.allpass_in = filtered;
        self.allpass_out = tuned;
        self.line[self.at] = tuned;
        self.at = i1;

        let lin = (1.0 - self.age.saturating_sub(self.hold) as f32 / self.release as f32).max(0.0);
        let release = lin * lin * (3.0 - 2.0 * lin);
        self.age += 1;
        // Velvet 5.5 ms finger-pad bloom eliminates click transients and harshness.
        let attack = (self.age as f32 / (RATE as f32 * 0.0055)).min(1.0);
        let bridge = s1 * release * attack;
        Some((
            [bridge * self.gain[0], bridge * self.gain[1]],
            bridge * f32::midpoint(self.gain[0], self.gain[1]) * self.body_send,
        ))
    }
}

/// Polyphonic Concert Grand Harp with tapered Sitka spruce soundboard resonators
/// (`110 Hz` chest warmth, `247 Hz` heart plate, `523 Hz` luminous crown) and
/// sympathetic `D3` / `A3` open-string bloom.
pub(super) struct ConcertHarp {
    strings: [HarpString; HARP_STRINGS],
    spruce_110: Resonator,
    spruce_247: Resonator,
    spruce_523: Resonator,
    symp_d3: Resonator,
    symp_a3: Resonator,
    plucks: u32,
    tail_frames: u32,
}

impl ConcertHarp {
    pub(super) fn new() -> Self {
        Self {
            strings: std::array::from_fn(|_| HarpString::new()),
            spruce_110: Resonator::new(110.0, 5.4),
            spruce_247: Resonator::new(246.942, 6.0),
            spruce_523: Resonator::new(523.251, 4.8),
            symp_d3: Resonator::new(146.832_38, 24.0),
            symp_a3: Resonator::new(220.0, 24.0),
            plucks: 0,
            tail_frames: 0,
        }
    }

    pub(super) fn pluck(&mut self, pitch: u8, seconds: f32, gain: [f32; 2], late: f32, cents: f32) {
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
        self.strings[slot].pluck(pitch, seconds, gain, late, cents, self.plucks);
        self.tail_frames = RATE * 2 / 3;
    }

    pub(super) fn release(&mut self) {
        for string in &mut self.strings {
            string.release();
        }
    }

    #[inline]
    fn step_soundboard(&mut self, bridge: f32) -> [f32; 2] {
        let low = self.spruce_110.step(bridge) * 0.28;
        let mid = self.spruce_247.step(bridge) * 0.25;
        let crown = self.spruce_523.step(bridge) * 0.16;
        let symp_d = self.symp_d3.step(bridge) * 0.028;
        let symp_a = self.symp_a3.step(bridge) * 0.024;
        [
            low * 1.05 + mid * 0.96 + crown * 0.90 + symp_d,
            low * 0.95 + mid * 1.04 + crown * 1.10 + symp_a,
        ]
    }

    #[inline]
    pub(super) fn frame(&mut self, dry: &mut [f32; 2], feed: &mut f32) {
        let mut any = false;
        let mut direct = [0.0_f32; 2];
        let mut bridge = 0.0_f32;
        for string in &mut self.strings {
            if let Some((pair, force)) = string.next() {
                any = true;
                direct[0] += pair[0];
                direct[1] += pair[1];
                bridge += force;
            }
        }
        if any {
            self.tail_frames = RATE * 2 / 3;
        } else if self.tail_frames == 0 {
            return;
        } else {
            self.tail_frames -= 1;
        }
        let body = self.step_soundboard(bridge);
        let l = direct[0] + body[0];
        let r = direct[1] + body[1];
        dry[0] += l;
        dry[1] += r;
        *feed += (l + r) * 0.26;
    }

    pub(super) fn render(&mut self, out: &mut [[f32; 2]], feed: &mut [f32]) {
        let any_sounding = self.strings.iter().any(|s| s.sounding);
        if !any_sounding && self.tail_frames == 0 {
            return;
        }
        for (frame, send) in out.iter_mut().zip(feed.iter_mut()) {
            self.frame(frame, send);
        }
    }
}
