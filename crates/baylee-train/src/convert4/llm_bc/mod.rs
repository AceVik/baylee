//! Explicit executed external-LLM BC labels. No neural behavior probabilities.
#![allow(missing_docs)]
use anyhow::{Context, ensure};
use baylee_engine::choice::PlayerAction;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

mod alignment;
pub mod schema;
#[cfg(test)]
mod tests;
mod validation;

pub fn sha(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;
    let mut out = String::new();
    for byte in Sha256::digest(bytes) {
        write!(out, "{byte:02x}").expect("String");
    }
    out
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmLabelProvenance4 {
    pub record_input_n: u64,
    pub model: String,
    pub executed_action: PlayerAction,
}
struct Input {
    seat: u8,
    action: PlayerAction,
    model: String,
    eligible: bool,
}
/// Verified every socket input, including zero-weight standing/fallback origins.
pub struct LlmBc4 {
    remaining: BTreeMap<u64, Input>,
}
impl LlmBc4 {
    /// Load and validate all external LLM provenance for one game.
    ///
    /// # Errors
    /// Returns an error if any hash, declaration, transcript or provenance check fails.
    pub fn load(run: &[u8], manifest: &[u8], record: &[u8], game: &str) -> Result<Self, String> {
        validation::load(run, manifest, record, game).map_err(|e| e.to_string())
    }
    pub(super) fn input(
        &mut self,
        n: u64,
        seat: u8,
        action: &PlayerAction,
        forced: bool,
    ) -> Result<Option<LlmLabelProvenance4>, String> {
        let input = self
            .remaining
            .remove(&n)
            .ok_or("Missing verified external input")?;
        if input.seat != seat || input.action != *action {
            return Err("External input identity mismatch".into());
        }
        Ok(
            (input.eligible && !forced && !matches!(action, PlayerAction::Concede)).then_some(
                LlmLabelProvenance4 {
                    record_input_n: n,
                    model: input.model,
                    executed_action: input.action,
                },
            ),
        )
    }
    pub(super) fn finish(&self) -> Result<(), String> {
        if self.remaining.is_empty() {
            Ok(())
        } else {
            Err("Unconsumed external input provenance".into())
        }
    }
}

fn json_lines(bytes: &[u8]) -> anyhow::Result<Vec<Value>> {
    bytes
        .split(|b| *b == b'\n')
        .filter(|v| !v.is_empty())
        .map(|v| serde_json::from_slice(v).context("LLM JSONL"))
        .collect()
}
fn artifact(technical: &Value, role: &str, digest: &str) -> anyhow::Result<()> {
    let found: Vec<_> = technical["artifacts"]
        .as_array()
        .context("Missing technical artifacts")?
        .iter()
        .filter(|v| v["role"] == role)
        .collect();
    ensure!(
        found.len() == 1 && found[0]["sha256"] == digest,
        "LLM technical artifact identity mismatch"
    );
    Ok(())
}
