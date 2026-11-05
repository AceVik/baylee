use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::collections::BTreeMap;

/// Whether `mind`, as a game's request asked for it or its record declares
/// it, is a teacher whose games may become LLM-BC data for `model`: the
/// first corpus's CLIs at effort low (Codex for Terra, agy for every other
/// model), `JetBrains`' Junie, or a model behind an API, Anthropic's or an
/// OpenAI-compatible one; the last two at whatever effort their profile
/// set. None declares a house level.
#[must_use]
pub fn teacher(mind: &Value, model: &str) -> bool {
    let cli = || match mind["provider"].as_str() {
        Some(tool @ ("agy" | "codex")) => {
            let expected = if model == "gpt-6.1-terra" {
                "codex"
            } else {
                "agy"
            };
            tool == expected && mind["effort"] == "low"
        }
        Some("junie") => mind.get("effort").is_none_or(Value::is_string),
        _ => false,
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
            .map(|r| &r["mind"])
            .collect::<Vec<_>>();
        ensure!(
            model.as_str().is_some_and(|model| chain_followed(
                &declarations,
                expected,
                requested_fallbacks(&bound, seat),
                model
            )),
            "Actual declared identity differs from requested identity"
        );
    }
    Ok(())
}

/// The fallbacks `receipt` asked for at `seat`, in chain order; none in a
/// receipt from before the chain.
#[must_use]
pub fn requested_fallbacks(receipt: &Value, seat: usize) -> &[Value] {
    receipt["requested_fallbacks"][seat]
        .as_array()
        .map_or(&[], Vec::as_slice)
}

/// Whether `declared`, one seat's declarations in the record's order,
/// follow what its request asked for: first `primary`, a teacher of
/// `model`; after it the mind playing again (a socket dialled again
/// declares it again) or a later one of `fallbacks`, each a teacher of its
/// own model, never one before it (a game never hands back); and at most
/// the house last, once the chain was used up, with nothing after it. The
/// house's answers are never labels; the teachers' before it stay.
#[must_use]
pub fn chain_followed(
    declared: &[&Value],
    primary: &Value,
    fallbacks: &[Value],
    model: &str,
) -> bool {
    let chain: Vec<&Value> = std::iter::once(primary).chain(fallbacks).collect();
    let Some((first, rest)) = declared.split_first() else {
        return false;
    };
    if **first != *primary || !teacher(primary, model) {
        return false;
    }
    let mut at = 0;
    let mut housed = false;
    for mind in rest {
        if housed {
            return false;
        }
        if mind["kind"] == "house" {
            housed = true;
            continue;
        }
        let Some(next) = (at..chain.len()).find(|&k| chain[k] == *mind) else {
            return false;
        };
        if next > 0 && !mind["model"].as_str().is_some_and(|own| teacher(mind, own)) {
            return false;
        }
        at = next;
    }
    true
}
