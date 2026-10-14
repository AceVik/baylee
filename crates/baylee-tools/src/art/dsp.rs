//! Float64 equivalents of the preparation math; ffmpeg only decodes audio.

// Frame counts and FFT bins are bounded by decoded audio/2^18; PCM casts follow
// explicit clipping. These casts describe sampled signals rather than rules.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use anyhow::{Result, ensure};

pub(super) const RATE: usize = 44_100;
const FFT: usize = 1 << 18;
const WINDOW: usize = 2048;

pub(super) struct Mono {
    pub samples: Vec<f64>,
    pub correlation: Option<f64>,
    pub method: &'static str,
}

// NumPy's contiguous float64 sum uses eight lanes and recursively splits blocks
// above 128 elements. Matching its reduction also keeps DC removal reproducible.
fn sum(x: &[f64]) -> f64 {
    if x.len() < 8 {
        return x.iter().fold(-0.0, |a, b| a + b);
    }
    if x.len() > 128 {
        let half = (x.len() / 2) & !7;
        return sum(&x[..half]) + sum(&x[half..]);
    }
    let mut lanes = [0.0; 8];
    lanes.copy_from_slice(&x[..8]);
    let mut i = 8;
    while i + 8 <= x.len() {
        for lane in 0..8 {
            lanes[lane] += x[i + lane];
        }
        i += 8;
    }
    let mut total = ((lanes[0] + lanes[1]) + (lanes[2] + lanes[3]))
        + ((lanes[4] + lanes[5]) + (lanes[6] + lanes[7]));
    for &value in &x[i..] {
        total += value;
    }
    total
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

pub(super) fn to_mono(interleaved: &[f64], channels: usize) -> Result<Mono> {
    ensure!(
        channels > 0 && !interleaved.is_empty() && interleaved.len().is_multiple_of(channels),
        "invalid decoded channel layout"
    );
    ensure!(
        interleaved.iter().all(|x| x.is_finite()),
        "non-finite decoded audio"
    );
    let left: Vec<_> = interleaved.iter().step_by(channels).copied().collect();
    if channels == 1 {
        return Ok(Mono {
            samples: left,
            correlation: None,
            method: "mono",
        });
    }
    let right: Vec<_> = interleaved
        .iter()
        .skip(1)
        .step_by(channels)
        .copied()
        .collect();
    let left_mean = sum(&left) / left.len() as f64;
    let right_mean = sum(&right) / right.len() as f64;
    let a: Vec<_> = left.iter().map(|x| x - left_mean).collect();
    let b: Vec<_> = right.iter().map(|x| x - right_mean).collect();
    let denominator = dot(&a, &a).sqrt() * dot(&b, &b).sqrt();
    let correlation = (denominator > 0.0).then(|| (dot(&a, &b) / denominator).clamp(-1.0, 1.0));
    let (samples, method) = if correlation.is_some_and(|corr| corr >= 0.5) {
        (
            left.iter()
                .zip(&right)
                .map(|(a, b)| (a + b) / 2.0)
                .collect(),
            "mid",
        )
    } else if dot(&left, &left) >= dot(&right, &right) {
        (left, "left")
    } else {
        (right, "right")
    };
    Ok(Mono {
        samples,
        correlation,
        method,
    })
}

pub(super) fn remove_dc_and_onset(x: &mut [f64]) -> Result<usize> {
    ensure!(!x.is_empty(), "empty recording");
    let mean = sum(x) / x.len() as f64;
    for value in &mut *x {
        *value -= mean;
    }
    let peak = x.iter().copied().map(f64::abs).fold(0.0, f64::max);
    ensure!(peak > 0.0, "silent recording");
    Ok(x.iter()
        .position(|x| x.abs() > peak * 0.012)
        .unwrap_or(0)
        .saturating_sub(220))
}

#[derive(Clone, Copy, Default)]
struct Complex {
    re: f64,
    im: f64,
}

fn fft(values: &mut [Complex]) {
    let n = values.len();
    debug_assert!(n.is_power_of_two());
    for i in 0..n {
        let reversed = i.reverse_bits() >> (usize::BITS - n.trailing_zeros());
        if i < reversed {
            values.swap(i, reversed);
        }
    }
    let mut size = 2;
    while size <= n {
        let half = size / 2;
        let roots: Vec<_> = (0..half)
            .map(|k| {
                let angle = -std::f64::consts::TAU * k as f64 / size as f64;
                Complex {
                    re: angle.cos(),
                    im: angle.sin(),
                }
            })
            .collect();
        for block in values.chunks_exact_mut(size) {
            for (i, root) in roots.iter().enumerate() {
                let a = block[i];
                let b = block[i + half];
                let re = root.re * b.re - root.im * b.im;
                let im = root.re * b.im + root.im * b.re;
                block[i] = Complex {
                    re: a.re + re,
                    im: a.im + im,
                };
                block[i + half] = Complex {
                    re: a.re - re,
                    im: a.im - im,
                };
            }
        }
        size *= 2;
    }
}

fn peak_near(spectrum: &[Complex], midi: i16) -> Result<(f64, f64)> {
    let want = 440.0 * 2.0_f64.powf(f64::from(midi - 69) / 12.0);
    let lo = (want * 0.94 * FFT as f64 / RATE as f64).ceil() as usize;
    let hi = (want * 1.06 * FFT as f64 / RATE as f64).ceil() as usize;
    ensure!(
        lo > 0 && lo < hi && hi <= FFT / 2 + 1,
        "pitch outside FFT range: MIDI {midi}"
    );
    let mut best = (lo, -1.0);
    for (k, value) in spectrum.iter().enumerate().take(hi).skip(lo) {
        let magnitude = value.re.hypot(value.im);
        if magnitude > best.1 {
            best = (k, magnitude);
        }
    }
    Ok((
        1200.0 * ((best.0 as f64 * RATE as f64 / FFT as f64) / want).log2(),
        best.1,
    ))
}

pub(super) fn measure(x: &[f64], midi: i16, onset: usize) -> Result<(f64, bool)> {
    let start = onset + RATE / 5;
    let window = if x.len() > start + RATE {
        &x[start..start + RATE]
    } else {
        &x[x.len().saturating_sub(RATE)..]
    };
    ensure!(window.len() > 1, "recording too short to measure");
    let mut spectrum = vec![Complex::default(); FFT];
    for (i, (&sample, value)) in window.iter().zip(&mut spectrum).enumerate() {
        value.re = sample
            * (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / (window.len() - 1) as f64).cos());
    }
    fft(&mut spectrum);
    let (cents, here) = peak_near(&spectrum, midi)?;
    let (_, below) = peak_near(&spectrum, midi - 12)?;
    Ok((cents, below < here * 0.25))
}

pub(super) fn find_loop(x: &[f64], start: usize, length: usize) -> Result<(usize, f64)> {
    ensure!(
        start + WINDOW <= x.len(),
        "recording shorter than loop reference"
    );
    let want = start + length;
    let first = (start + RATE / 2).max(want.saturating_sub(RATE / 4));
    let last = x.len().saturating_sub(WINDOW + 1).min(want + RATE / 4);
    ensure!(first < last, "recording has no valid loop end candidates");
    let reference = &x[start..start + WINDOW];
    let norm = dot(reference, reference).sqrt();
    let mut energy = dot(&x[first..first + WINDOW], &x[first..first + WINDOW]);
    let (mut best, mut end) = (-2.0, want);
    for candidate in first..last {
        let numerator = dot(reference, &x[candidate..candidate + WINDOW]);
        let mut c = numerator / (norm * energy.max(0.0).sqrt() + 1e-12);
        // Recompute near ties directly, so accumulated sliding-energy error
        // cannot choose a different cycle of an almost-periodic recording.
        if (c - best).abs() < 1e-10 {
            let segment = &x[candidate..candidate + WINDOW];
            c = numerator / (norm * dot(segment, segment).sqrt() + 1e-12);
            let previous = &x[end..end + WINDOW];
            best = dot(reference, previous) / (norm * dot(previous, previous).sqrt() + 1e-12);
        }
        if c > best {
            best = c;
            end = candidate;
        }
        // Sliding energy avoids recomputing a second 2048-sample dot per candidate.
        energy += x[candidate + WINDOW].powi(2) - x[candidate].powi(2);
    }
    Ok((end, best))
}

pub(super) fn bake_loop(x: &[f64], start: usize, end: usize) -> Result<Vec<f64>> {
    ensure!(
        start < end && end < x.len(),
        "invalid loop bounds {start}..{end}"
    );
    let crossfade = (RATE * 6 / 100).min(start).min(end - start);
    let mut out = x[..=end].to_vec();
    for i in 0..crossfade {
        let t = i as f64 / crossfade as f64;
        out[end - crossfade + i] =
            x[end - crossfade + i] * (1.0 - t) + x[start - crossfade + i] * t;
    }
    out[end] = out[start];
    Ok(out)
}

pub(super) fn one_shot(x: &[f64], keep: usize) -> Result<Vec<f64>> {
    let mut out = x[..x.len().min(keep)].to_vec();
    ensure!(out.len() >= 32, "one-shot shorter than 32-frame attack");
    let n = (RATE / 4).min(out.len() / 4);
    let len = out.len();
    for i in 0..n {
        out[len - n + i] *= (1.0 - i as f64 / (n - 1) as f64).powi(2);
    }
    for (i, sample) in out.iter_mut().take(32).enumerate() {
        *sample *= i as f64 / 31.0;
    }
    Ok(out)
}

pub(super) fn pcm16(x: &[f64]) -> Result<Vec<u8>> {
    ensure!(
        x.iter().all(|x| x.is_finite()),
        "non-finite prepared samples"
    );
    let peak = x.iter().copied().map(f64::abs).fold(0.0, f64::max);
    ensure!(
        peak.is_finite() && peak > 0.0,
        "empty, silent or non-finite prepared sample"
    );
    let scale = 0.89 / peak;
    Ok(x.iter()
        .flat_map(|&sample| {
            // NumPy round uses ties-to-even, unlike Rust's ordinary round().
            (((sample * scale).clamp(-1.0, 1.0) * 32767.0).round_ties_even() as i16).to_le_bytes()
        })
        .collect())
}

pub(super) fn rounded(value: f64, digits: i32) -> f64 {
    let scale = 10.0_f64.powi(digits);
    (value * scale).round_ties_even() / scale
}

#[cfg(test)]
// Exact equality is intentional for seam samples and authored JSON constants.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn tone(hz: f64, frames: usize) -> Vec<f64> {
        (0..frames)
            .map(|i| (std::f64::consts::TAU * hz * i as f64 / RATE as f64).sin())
            .collect()
    }

    #[test]
    fn stereo_policy_protects_anticorrelated_pairs() {
        let source = tone(440.0, 4096);
        let mid: Vec<_> = source.iter().flat_map(|&x| [x, x * 0.5]).collect();
        assert_eq!(to_mono(&mid, 2).unwrap().method, "mid");
        let opposed: Vec<_> = source.iter().flat_map(|&x| [x * 0.5, -x]).collect();
        let mono = to_mono(&opposed, 2).unwrap();
        assert_eq!(mono.method, "right");
        assert!(mono.correlation.unwrap() < -0.999);
        assert!(
            mono.samples
                .iter()
                .zip(source)
                .all(|(a, b)| (*a + b).abs() < 1e-12)
        );
    }

    #[test]
    fn fft_matches_direct_dft() {
        let signal: Vec<_> = (0..32)
            .map(|i| Complex {
                re: (f64::from(i) * 0.3).sin(),
                im: 0.0,
            })
            .collect();
        let mut actual = signal.clone();
        fft(&mut actual);
        for (k, value) in actual.iter().enumerate() {
            let mut expected = Complex::default();
            for (i, sample) in signal.iter().enumerate() {
                let phase = -std::f64::consts::TAU * (k * i) as f64 / signal.len() as f64;
                expected.re += sample.re * phase.cos();
                expected.im += sample.re * phase.sin();
            }
            assert!((value.re - expected.re).abs() < 1e-12);
            assert!((value.im - expected.im).abs() < 1e-12);
        }
    }

    #[test]
    fn measured_pitch_and_octave_guard_use_signal() {
        let signal = tone(440.0 * 2.0_f64.powf(-35.0 / 1200.0), RATE * 2);
        let (cents, clean) = measure(&signal, 69, 0).unwrap();
        assert!((cents + 35.0).abs() < 1.0 && clean);
        let wrong = tone(220.0, RATE * 2);
        assert!(!measure(&wrong, 69, 0).unwrap().1);
    }

    #[test]
    fn sliding_loop_search_agrees_with_naive_reference() {
        let mut state = 17_u64;
        let mut x: Vec<_> = (0..35_000)
            .map(|_| {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                (state >> 32) as f64 / f64::from(u32::MAX) - 0.5
            })
            .collect();
        let start = 1500;
        let expected_end = 28_000;
        let reference = x[start..start + WINDOW].to_vec();
        x[expected_end..expected_end + WINDOW].copy_from_slice(&reference);
        let (end, correlation) = find_loop(&x, start, 27_000).unwrap();
        let want = start + 27_000;
        let first = (start + RATE / 2).max(want - RATE / 4);
        let last = (x.len() - WINDOW - 1).min(want + RATE / 4);
        let reference_norm = dot(&reference, &reference).sqrt();
        let mut naive = (first, -2.0);
        for candidate in first..last {
            let segment = &x[candidate..candidate + WINDOW];
            let value =
                dot(&reference, segment) / (reference_norm * dot(segment, segment).sqrt() + 1e-12);
            if value > naive.1 {
                naive = (candidate, value);
            }
        }
        assert_eq!(end, naive.0);
        assert!((correlation - naive.1).abs() < 1e-10);
        assert_eq!(end, expected_end);
        assert!((correlation - 1.0).abs() < 1e-10);
        let baked = bake_loop(&x, start, end).unwrap();
        assert_eq!(baked.len(), end + 1);
        assert_eq!(baked[start], baked[end]);
        let bytes = pcm16(&baked).unwrap();
        assert_eq!(
            &bytes[start * 2..start * 2 + 2],
            &bytes[end * 2..end * 2 + 2]
        );
    }

    #[test]
    fn silence_is_refused_and_one_shots_fade() {
        assert!(remove_dc_and_onset(&mut [0.0; 100]).is_err());
        assert!(pcm16(&[0.0; 100]).is_err());
        let out = one_shot(&tone(120.0, RATE), RATE).unwrap();
        assert_eq!(out[0], 0.0);
        assert_eq!(*out.last().unwrap(), 0.0);
        let pcm = pcm16(&out).unwrap();
        let peak = pcm
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_le_bytes([b[0], b[1]]).unsigned_abs())
            .max()
            .unwrap();
        assert_eq!(peak, (32767.0_f64 * 0.89).round_ties_even() as u16);
    }

    #[test]
    fn quantization_uses_ties_to_even_and_little_endian() {
        let bytes = pcm16(&[
            0.89,
            0.5 / 32767.0,
            1.5 / 32767.0,
            2.5 / 32767.0,
            -1.5 / 32767.0,
        ])
        .unwrap();
        let values: Vec<_> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect();
        assert_eq!(&values[1..], &[0, 2, 2, -2]);
        assert!(pcm16(&[1.0, f64::NAN]).is_err());
    }

    #[test]
    fn dc_is_removed_before_the_original_onset_margin() {
        let mut x = vec![0.2; 2000];
        x.extend(tone(440.0, RATE).into_iter().map(|sample| sample + 0.2));
        let onset = remove_dc_and_onset(&mut x).unwrap();
        assert!((1770..1790).contains(&onset));
        assert!((sum(&x) / x.len() as f64).abs() < 1e-12);
    }
}
