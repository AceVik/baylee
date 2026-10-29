//! Host statistics for tensor benchmarks, preserving each historical quantile rule.
use super::{floats, ratio};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
pub(super) fn rounded(value: f64, digits: usize) -> Result<f64> {
    Ok(format!("{value:.digits$}").parse()?)
}
fn linear(values: &[f64], q: f64) -> f64 {
    let index = ratio(values.len() - 1) * q;
    let lo = index.floor();
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "Index is bounded by the validated vector length."
    )]
    let left = lo as usize;
    let right = (left + 1).min(values.len() - 1);
    let fraction = index - lo;
    let difference = values[right] - values[left];
    if fraction >= 0.5 {
        values[right] - difference * (1.0 - fraction)
    } else {
        values[left] + difference * fraction
    }
}
pub(super) fn statistics(input: &Value) -> Result<Value> {
    let mut samples = floats(&input["samples"])?;
    ensure!(
        !samples.is_empty() && samples.iter().all(|v| *v >= 0.0),
        "nonempty nonnegative benchmark timings required"
    );
    let mode = input["mode"].as_str().context("benchmark mode")?;
    if mode == "training" {
        let elapsed = python_sum(&samples);
        ensure!(elapsed > 0.0, "positive timing total required");
        return Ok(
            json!({"updates_per_second":1000.0/(numpy_sum(&samples)/ratio(samples.len())),"valid_frames_per_second":input["frames"].as_f64().context("frames")?/(elapsed/1000.0),"decisions_per_second":input["decisions"].as_f64().context("decisions")?/(elapsed/1000.0)}),
        );
    }
    samples.sort_by(f64::total_cmp);
    let n = samples.len();
    #[allow(
        clippy::manual_midpoint,
        reason = "Keep the historical NumPy/statistics even median arithmetic, including rounding, instead of changing it to f64::midpoint."
    )]
    let median = if n.is_multiple_of(2) {
        (samples[n / 2 - 1] + samples[n / 2]) / 2.0
    } else {
        samples[n / 2]
    };
    match mode {
        "inference" => Ok(json!({"p50_ms":median,"p95_ms":linear(&samples,0.95)})),
        "t1" => Ok(
            json!({"p50_ms":rounded(linear(&samples,0.5),2)?,"p95_ms":rounded(linear(&samples,0.95),2)?,"p99_ms":rounded(linear(&samples,0.99),2)?}),
        ),
        "step" => {
            ensure!(median > 0.0, "positive median required");
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "The historical indexed p95 uses truncation of len*0.95, within the vector."
            )]
            let index = (ratio(n) * 0.95) as usize;
            let decisions = input["batch"].as_f64().context("batch")? / median;
            ensure!(
                decisions >= 0.0 && decisions < f64::from(u32::MAX),
                "throughput outside supported report range"
            );
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "Validated nonnegative throughput is below u32::MAX; ties-even matches Python round."
            )]
            let throughput = decisions.round_ties_even() as u32;
            Ok(
                json!({"median_ms":rounded(median*1000.0,3)?,"p95_ms":rounded(samples[index]*1000.0,3)?,"decisions_per_s":throughput}),
            )
        }
        other => anyhow::bail!("unknown benchmark mode {other}"),
    }
}
pub(super) fn plan(input: &Value) -> Result<Value> {
    match input["mode"].as_str().context("plan mode")? {
        "rows" => {
            let length = input["length"].as_u64().context("dataset length")?;
            ensure!(length > 0, "nonempty inference dataset required");
            let length = usize::try_from(length)?;
            let step = ratio(length - 1) / 119.0;
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "Nonnegative linspace positions are bounded by the dataset length; floor matches NumPy dtype=int."
            )]
            let mut rows: Vec<_> = (0..120)
                .map(|i| (ratio(i) * step).floor() as usize)
                .collect();
            rows[119] = length - 1;
            Ok(json!(rows))
        }
        "select" => {
            let entities: Vec<u64> = serde_json::from_value(input["entities"].clone())?;
            ensure!(!entities.is_empty(), "no real decision frames sampled");
            let mut order: Vec<_> = (0..entities.len()).collect();
            order.sort_by_key(|&i| entities[i]);
            let mut indices = vec![0, order.len() / 2, order.len() - 1];
            indices.sort_unstable();
            indices.dedup();
            Ok(json!(
                indices.into_iter().map(|i| order[i]).collect::<Vec<_>>()
            ))
        }
        other => anyhow::bail!("unknown benchmark plan {other}"),
    }
}

// NumPy contiguous floating reductions use eight lanes and recursive 128-value
// blocks. Preserve the old np.mean rounding, distinct from Python sum timing.
pub(super) fn numpy_sum<T>(values: &[T]) -> T
where
    T: Copy + From<f32> + std::ops::Add<Output = T> + std::ops::AddAssign,
{
    let n = values.len();
    if n < 8 {
        return values.iter().fold(T::from(-0.0), |a, b| a + *b);
    }
    if n > 128 {
        let half = (n / 2) / 8 * 8;
        return numpy_sum(&values[..half]) + numpy_sum(&values[half..]);
    }
    let mut lanes = [T::from(0.0); 8];
    lanes.copy_from_slice(&values[..8]);
    let mut at = 8;
    while at + 8 <= n {
        for i in 0..8 {
            lanes[i] += values[at + i];
        }
        at += 8;
    }
    let mut total = ((lanes[0] + lanes[1]) + (lanes[2] + lanes[3]))
        + ((lanes[4] + lanes[5]) + (lanes[6] + lanes[7]));
    for value in &values[at..] {
        total += *value;
    }
    total
}

// CPython 3.12+ sum(float) uses a compensated high/low accumulator.
fn python_sum(values: &[f64]) -> f64 {
    let (mut high, mut low) = (0.0_f64, 0.0_f64);
    for &value in values {
        let total = high + value;
        low += if high.abs() >= value.abs() {
            (high - total) + value
        } else {
            (value - total) + high
        };
        high = total;
    }
    if low.is_finite() { high + low } else { high }
}
