//! Native planning, allocation estimates and reports around CPU tensor measurements.
#[cfg(target_os = "linux")]
pub mod cli;
mod measurement;
mod report;
mod shapes;
#[cfg(test)]
mod tests;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
/// Evenly spaced deterministic lower integer positions, including both endpoints.
/// # Errors
/// Refuses an empty epoch or nonpositive count.
pub fn indices(length: u64, count: u64) -> Result<Vec<u64>> {
    ensure!(
        length > 0 && count > 0,
        "nonempty epoch and positive sample count required"
    );
    let count = count.min(length);
    if count == 1 {
        return Ok(vec![0]);
    }
    (0..count)
        .map(|i| {
            Ok(u64::try_from(
                u128::from(i) * u128::from(length - 1) / u128::from(count - 1),
            )?)
        })
        .collect()
}
/// Small JSON RPC used only by the associated Torch worker.
/// # Errors
/// Rejects malformed measurements, unsafe budgets, source mutation or existing reports.
pub fn boundary(value: &Value) -> Result<Value> {
    match value["kind"].as_str().context("batch audit request kind")? {
        "indices" => Ok(json!(indices(
            value["length"].as_u64().context("epoch length")?,
            value["count"].as_u64().context("sample count")?
        )?)),
        "estimate" => shapes::estimate(value),
        "finish" => report::finish(value),
        "worker-error" => {
            crate::files::atomic_json(
                std::path::Path::new(
                    value["diagnostic"]
                        .as_str()
                        .context("worker diagnostic path")?,
                ),
                &json!({"error_type":value["error_type"],"detail":value["detail"]}),
            )?;
            Ok(json!({"recorded":true}))
        }
        other => anyhow::bail!("unknown batch audit request {other}"),
    }
}
