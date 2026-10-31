use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::collections::BTreeMap;

/// Whether `mind`, as a game's request asked for it or its record declares
/// it, is a teacher whose games may become LLM-BC data for `model`: a CLI
/// at effort low (Codex for Terra, agy for every other model), or a model
/// behind an API, Anthropic's or an OpenAI-compatible one, at whatever
/// effort its profile set. Neither declares a house level.
#[must_use]
pub fn teacher(mind: &Value, model: &str) -> bool {
    let cli = || {
        let tool = if model == "gpt-6.1-terra" {
            "codex"
        } else {
            "agy"
        };
        mind["provider"] == tool && mind["effort"] == "low"
    };
    let api = || {
        matches!(mind["provider"].as_str(), Some("anthropic" | "openai"))
            && mind.get("effort").is_none_or(Value::is_string)
    };
    mind["model"] == model
        && mind.get("level").is_none_or(|v| v == "")
        && match mind["kind"].as_str() {
            Some("llm_cli") => cli(),
            Some("llm_api") => api(),
            _ => false,
        }
}

pub(super) fn verify(
    request: &Value,
    records: &[Value],
    artifacts: &BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    let models = request["models"]
        .as_array()
        .filter(|m| m.len() == 2)
        .context("Two declared model seats are required")?;
    let bound = artifacts
        .get("input_receipt")
        .map(|raw| serde_json::from_slice::<Value>(raw))
        .transpose()?;
    let non_agy = records
        .iter()
        .any(|r| r["kind"] == "declared_mind" && r["mind"]["provider"] != "agy");
    let terra = models.iter().any(|m| m == "gpt-6.1-terra");
    ensure!(
        !(terra || non_agy) || bound.is_some(),
        "Codex Terra and API teachers require the requested identity receipt"
    );
    let Some(bound) = bound else {
        return Ok(());
    };
    if bound.get("requested_minds").is_none() && !non_agy && !terra {
        return Ok(());
    }
    ensure!(
        bound["requested_models"] == request["models"]
            && bound["requested_minds"]
                .as_array()
                .is_some_and(|v| v.len() == 2),
        "Requested identity receipt/model mismatch"
    );
    for (seat, model) in models.iter().enumerate() {
        let expected = &bound["requested_minds"][seat];
        ensure!(
            model.as_str().is_some_and(|model| teacher(expected, model)),
            "Unsupported or mismatched requested teacher identity"
        );
        let declarations = records
            .iter()
            .filter(|r| r["kind"] == "declared_mind" && r["seat"].as_u64() == Some(seat as u64))
            .collect::<Vec<_>>();
        ensure!(
            !declarations.is_empty() && declarations.iter().all(|r| r["mind"] == *expected),
            "Actual declared identity differs from requested identity"
        );
    }
    Ok(())
}
