//! Offline matched evaluation statistics and frozen-job verification.
pub mod calibration;
pub mod diagnostics;
mod paired;
mod summary;
use crate::files::hex;
pub use crate::files::{atomic_json, read_json};
use anyhow::{Context, Result};
pub use paired::{compare, read_verified};
use serde_json::Value;
pub use summary::summarize;

fn number(value: &Value, key: &str) -> Result<f64> {
    let n = value[key]
        .as_f64()
        .with_context(|| format!("missing numeric {key}"))?;
    anyhow::ensure!(n.is_finite(), "nonfinite {key}");
    Ok(n)
}

fn flag(value: &Value, key: &str) -> Result<bool> {
    value[key]
        .as_bool()
        .with_context(|| format!("missing boolean {key}"))
}
fn case_key(row: &Value, focal: bool) -> Result<(u64, u64, Vec<String>, u64)> {
    let decks = row["decks"]
        .as_array()
        .context("missing decks")?
        .iter()
        .map(|v| v.as_str().map(str::to_owned).context("invalid deck"))
        .collect::<Result<_>>()?;
    Ok((
        row["table"].as_u64().context("missing table")?,
        row["seed"].as_u64().context("missing seed")?,
        decks,
        if focal {
            row["focal_seat"].as_u64().context("missing focal seat")?
        } else {
            0
        },
    ))
}

pub mod live_import;
pub mod maintenance;
pub mod progress;
pub mod publication;
#[cfg(test)]
mod publication_tests;
#[cfg(test)]
mod tests;
