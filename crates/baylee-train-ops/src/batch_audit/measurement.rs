//! Validate Tensor counts and derive padding/attention report ratios natively.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
fn count(v: &Value, key: &str) -> Result<u64> {
    v[key]
        .as_u64()
        .with_context(|| format!("invalid tensor count {key}"))
}
fn fraction(actual: u64, allocated: u64) -> Result<Value> {
    ensure!(actual <= allocated, "tensor count exceeds allocation");
    if allocated == 0 {
        return Ok(Value::Null);
    }
    Ok(json!(1.0 - as_float(actual) / as_float(allocated)))
}
pub(super) fn normalize(record: &Value) -> Result<Value> {
    let estimate = &record["estimate"];
    let enabled = estimate["measure"]
        .as_bool()
        .context("native measurement budget decision")?;
    let mut result = json!({"batch_index":record["batch_index"],"segments":record["segments"],"predicted_collated_bytes":estimate["predicted_collated_bytes"],"conservative_tensor_peak_bytes":estimate["conservative_tensor_peak_bytes"],"skipped":!enabled});
    if !enabled {
        ensure!(
            record["measurement"].is_null(),
            "over-budget tensor batch was measured"
        );
        result["reason"] =
            json!("Conservative tensor allocation estimate exceeds per-batch budget");
        return Ok(result);
    }
    let measured = &record["measurement"];
    let fields = measured.as_object().context("missing tensor measurement")?;
    let b = count(measured, "batch_size")?;
    let u = count(measured, "batch_unroll")?;
    let bu = b.checked_mul(u).context("batch geometry overflow")?;
    ensure!(bu > 0, "nonempty tensor batch required");
    let frames = count(measured, "valid_frames")?;
    let active = count(measured, "active_timesteps")?;
    let rows = count(measured, "thinking_decision_rows")?;
    ensure!(
        frames <= bu && active <= u && rows <= frames,
        "invalid tensor frame counts"
    );
    let decision_total = (0..4).try_fold(0u64, |total, head| {
        total
            .checked_add(
                measured["decision_head_counts"][head.to_string()]
                    .as_u64()
                    .context("decision head count")?,
            )
            .context("head count overflow")
    })?;
    ensure!(decision_total == rows, "unclassified tensor decision head");
    result
        .as_object_mut()
        .context("report object")?
        .extend(fields.clone());
    let denominator = b
        .checked_mul(active)
        .context("active tensor geometry overflow")?
        .max(1);
    result["thinking_row_fraction_B_active_time"] = json!(as_float(rows) / as_float(denominator));
    for name in ["entities", "edges", "candidates", "pairs"] {
        let item = &measured[name];
        let width = count(item, "padded_width")?;
        let allocated = count(item, "allocated_entries")?;
        let actual = count(item, "actual_entries")?;
        ensure!(
            allocated == bu.checked_mul(width).context("mask geometry overflow")?
                && count(item, "max_actual")? <= width,
            "tensor mask geometry differs"
        );
        result[name]["padding_fraction"] = fraction(actual, allocated)?;
    }
    let width = count(&measured["entities"], "padded_width")?;
    let allocated = bu
        .checked_mul(width)
        .and_then(|v| v.checked_mul(width))
        .context("attention geometry overflow")?;
    result["entity_attention_E2"]["allocated_entries"] = json!(allocated);
    result["entity_attention_E2"]["padding_fraction"] = fraction(
        count(&measured["entity_attention_E2"], "actual_entries")?,
        allocated,
    )?;
    Ok(result)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "Only report ratios use approximate floats; all allocation counts remain exact u64."
)]
fn as_float(value: u64) -> f64 {
    value as f64
}
