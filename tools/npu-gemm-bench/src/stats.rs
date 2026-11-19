//! Timing summaries.

use std::time::Duration;

/// Percentiles of a set of per-call times, in milliseconds.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Summary {
    pub n: usize,
    pub min_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
}

impl Summary {
    pub fn of(times: &[Duration]) -> Self {
        assert!(!times.is_empty(), "no samples");
        let mut ms: Vec<f64> = times.iter().map(|t| t.as_secs_f64() * 1e3).collect();
        ms.sort_by(f64::total_cmp);
        Self {
            n: ms.len(),
            min_ms: ms[0],
            p50_ms: percentile(&ms, 0.50),
            p95_ms: percentile(&ms, 0.95),
            max_ms: ms[ms.len() - 1],
            mean_ms: ms.iter().sum::<f64>() / ms.len() as f64,
        }
    }
}

/// Linear interpolation between the closest ranks of a sorted slice.
fn percentile(sorted: &[f64], q: f64) -> f64 {
    let pos = q * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo as f64)
}

/// GFLOP/s of an `m x k x n` GEMM (2mkn FLOP) that took `ms` milliseconds.
pub fn gflops(m: usize, k: usize, n: usize, ms: f64) -> f64 {
    2.0 * m as f64 * k as f64 * n as f64 / (ms * 1e6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_interpolate() {
        let t: Vec<Duration> = (1..=101).map(Duration::from_millis).collect();
        let s = Summary::of(&t);
        assert!((s.p50_ms - 51.0).abs() < 1e-9);
        assert!((s.p95_ms - 96.0).abs() < 1e-9);
        assert!((s.min_ms - 1.0).abs() < 1e-9);
        assert!((s.max_ms - 101.0).abs() < 1e-9);
    }

    #[test]
    fn gflops_counts_two_flops_per_mac() {
        assert!((gflops(1000, 1000, 1000, 1000.0) - 2.0).abs() < 1e-12);
    }
}
