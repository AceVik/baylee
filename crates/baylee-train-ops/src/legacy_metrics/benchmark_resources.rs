//! Unit conversions and throughput presentation for retained tensor probes.
use super::benchmark::rounded;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
fn number(input: &Value, key: &str) -> Result<f64> {
    input[key]
        .as_f64()
        .filter(|n| n.is_finite())
        .with_context(|| format!("benchmark {key}"))
}
fn integer(value: f64) -> Result<u64> {
    ensure!(
        (0.0..=f64::from(u32::MAX)).contains(&value),
        "benchmark integer report out of range"
    );
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "Validated bounded nonnegative result, with Python round ties-even."
    )]
    Ok(u64::from(value.round_ties_even() as u32))
}
pub(super) fn run(input: &Value) -> Result<Value> {
    let mode = input["mode"].as_str().context("benchmark resource mode")?;
    if mode == "speedup" {
        let gpu = number(input, "gpu")?;
        ensure!(gpu > 0.0, "positive GPU median required");
        return Ok(json!(number(input, "cpu")? * number(input, "batch")? / gpu));
    }
    if mode == "parameters" {
        return Ok(json!(rounded(number(input, "parameters")? / 1e6, 2)?));
    }
    let elapsed = number(input, "elapsed")?;
    ensure!(
        elapsed > 0.0,
        "positive elapsed benchmark duration required"
    );
    match mode {
        "step-training" => Ok(
            json!({"steps_per_s":rounded(number(input,"steps")?/elapsed,2)?,"samples_per_s":integer(number(input,"steps")?*number(input,"batch")?/elapsed)?,"peak_gb":rounded(number(input,"peak_bytes")?/2_f64.powi(30),2)?}),
        ),
        "t1-training" => {
            let each = elapsed / number(input, "iterations")?;
            ensure!(each > 0.0, "positive iteration time required");
            Ok(
                json!({"params_m":rounded(number(input,"parameters")?/1e6,2)?,"peak_vram_mb":rounded(number(input,"peak_bytes")?/2_f64.powi(20),1)?,"step_ms":rounded(each*1000.0,1)?,"decision_steps_per_s":rounded((number(input,"batch")?*number(input,"unroll")?)/each,1)?}),
            )
        }
        other => anyhow::bail!("unknown benchmark resource mode {other}"),
    }
}
