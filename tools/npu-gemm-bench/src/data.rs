//! Test matrices, bf16 rounding and the error metrics.

/// Rounds an f32 to the nearest bf16 (ties to even), as raw bits.
pub fn f32_to_bf16(x: f32) -> u16 {
    let bits = x.to_bits();
    if x.is_nan() {
        return ((bits >> 16) | 0x40) as u16;
    }
    let round = 0x7FFF + ((bits >> 16) & 1);
    (bits.wrapping_add(round) >> 16) as u16
}

/// Widens bf16 bits to the f32 they stand for.
pub fn bf16_to_f32(b: u16) -> f32 {
    f32::from_bits(u32::from(b) << 16)
}

/// A seeded splitmix64 stream; the benchmark's inputs are reproducible.
pub struct SplitMix(u64);

impl SplitMix {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [-1, 1).
    pub fn uniform(&mut self) -> f32 {
        let unit = (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32;
        unit * 2.0 - 1.0
    }
}

/// A row-major matrix held as bf16 bits, with the same values widened to
/// f32 for the reference.
pub struct Bf16Matrix {
    pub bits: Vec<u16>,
    pub wide: Vec<f32>,
}

impl Bf16Matrix {
    /// `len` uniform values in `[-scale, scale)`, rounded to bf16.
    pub fn random(len: usize, scale: f32, rng: &mut SplitMix) -> Self {
        let bits: Vec<u16> = (0..len).map(|_| f32_to_bf16(rng.uniform() * scale)).collect();
        let wide = bits.iter().map(|&b| bf16_to_f32(b)).collect();
        Self { bits, wide }
    }
}

/// How far `got` lies from `want`.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct ErrorStats {
    /// max |got - want|.
    pub max_abs: f64,
    /// max |got - want| / |want| over elements with |want| >= 1% of
    /// max |want| (smaller references make the ratio meaningless).
    pub max_rel: f64,
    /// ||got - want||_F / ||want||_F.
    pub rel_fro: f64,
    /// max |want|, for scale.
    pub max_ref: f64,
    /// Elements that are NaN or infinite in `got`.
    pub non_finite: usize,
}

pub fn compare(got: &[f32], want: &[f64]) -> ErrorStats {
    assert_eq!(got.len(), want.len(), "compare: length mismatch");
    let max_ref = want.iter().fold(0.0_f64, |m, w| m.max(w.abs()));
    let floor = max_ref * 0.01;
    let (mut max_abs, mut max_rel, mut d2, mut r2, mut non_finite) = (0.0_f64, 0.0_f64, 0.0, 0.0, 0);
    for (&g, &w) in got.iter().zip(want) {
        if !g.is_finite() {
            non_finite += 1;
            continue;
        }
        let d = (f64::from(g) - w).abs();
        max_abs = max_abs.max(d);
        if w.abs() >= floor && w != 0.0 {
            max_rel = max_rel.max(d / w.abs());
        }
        d2 += d * d;
        r2 += w * w;
    }
    ErrorStats {
        max_abs,
        max_rel,
        rel_fro: if r2 > 0.0 { (d2 / r2).sqrt() } else { 0.0 },
        max_ref,
        non_finite,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bf16_round_trip_and_ties() {
        for x in [0.0_f32, 1.0, -2.5, 3.140_625, 1e-3, -65504.0] {
            let r = bf16_to_f32(f32_to_bf16(x));
            assert!((r - x).abs() <= x.abs() * 2.0_f32.powi(-8), "{x} -> {r}");
        }
        // 1 + 2^-8 is halfway between 1 and 1 + 2^-7: ties to even gives 1.
        assert_eq!(bf16_to_f32(f32_to_bf16(1.0 + 2.0_f32.powi(-8))), 1.0);
        // 1 + 3 * 2^-8 ties up to the even 1 + 2^-6.
        assert_eq!(
            bf16_to_f32(f32_to_bf16(1.0 + 3.0 * 2.0_f32.powi(-8))),
            1.0 + 2.0_f32.powi(-6)
        );
        assert!(bf16_to_f32(f32_to_bf16(f32::NAN)).is_nan());
    }

    #[test]
    fn uniform_stays_in_range() {
        let mut rng = SplitMix::new(7);
        for _ in 0..10_000 {
            let u = rng.uniform();
            assert!((-1.0..1.0).contains(&u));
        }
    }

    #[test]
    fn compare_reports_relative_error() {
        let e = compare(&[1.0, 2.0, 0.0], &[1.0, 2.5, 1e-9]);
        assert!((e.max_abs - 0.5).abs() < 1e-12);
        assert!((e.max_rel - 0.2).abs() < 1e-12);
        assert_eq!(e.non_finite, 0);
    }
}
