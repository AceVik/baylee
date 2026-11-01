use super::{case_key, flag, number, read_json, summarize};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

/// Bind a completed evaluation to its recorded executable and frozen inputs.
///
/// # Errors
/// Rejects incomplete jobs, missing provenance and mismatched model or paths.
pub fn read_verified(evaluation: &Path, job: &Path) -> Result<(Value, Value, String)> {
    let bytes = std::fs::read(evaluation)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    let run = read_json(job)?;
    ensure!(
        run["status"] == "complete",
        "Only completed evaluation jobs can be compared"
    );
    let args = run["command"].as_array().context("missing command")?;
    let mut options = BTreeMap::new();
    let mut i = 1;
    while i < args.len() {
        let key = args[i].as_str().context("invalid command argument")?;
        i += 1;
        let v = if [
            "--legacy-v3",
            "--fair-scouting",
            "--no-baseline",
            "--held-out-cards",
            "--raw-policy",
            "--catalog-superset",
            "--catalog-drift",
        ]
        .contains(&key)
        {
            json!(true)
        } else {
            let v = args.get(i).context("missing command option value")?.clone();
            i += 1;
            v
        };
        options.insert(key, v);
    }
    let out = options
        .get("--out")
        .and_then(Value::as_str)
        .context("missing --out")?;
    ensure!(
        Path::new(out).canonicalize()? == evaluation.canonicalize()?
            && options.get("--model") == value.get("model"),
        "Evaluation does not belong to the supplied job"
    );
    let manifest = value["decks_manifest"]
        .as_str()
        .filter(|s| !s.is_empty())
        .context("A frozen deck manifest is required for verified comparisons")?;
    ensure!(
        options.get("--decks-manifest").and_then(Value::as_str) == Some(manifest),
        "A frozen deck manifest is required for verified comparisons"
    );
    let integer = |key, default| -> Result<u64> {
        options.get(key).map_or(Ok(default), |v| {
            v.as_str()
                .context("invalid integer option")?
                .parse()
                .context("invalid integer option")
        })
    };
    ensure!(
        run["executable_sha256"].is_string() && run["inputs"][manifest].is_string(),
        "missing frozen input digest"
    );
    let provenance = json!({"engine_sha256":run["executable_sha256"],"deck_manifest_sha256":run["inputs"][manifest],
        "max_decisions":integer("--max-decisions",5000)?,"fair_scouting":options.get("--fair-scouting").unwrap_or(&json!(true)),
        "holdout_tier":options.get("--holdout-tier").unwrap_or(&json!("")),"generated":integer("--generated",32)?});
    Ok((value, provenance, super::hex(&Sha256::digest(bytes))))
}

/// Compare every matched case, enforcing equal protocol and house baselines.
///
/// # Errors
/// Rejects incompatible protocols, incomplete cases and changed house outcomes.
pub fn compare(
    candidate: &Value,
    reference: &Value,
    cp: &Value,
    rp: &Value,
    resamples: usize,
    allow_thinking_difference: bool,
) -> Result<Value> {
    check_protocol(candidate, reference, cp, rp, allow_thinking_difference)?;
    let left = index(candidate)?;
    let right = index(reference)?;
    ensure!(
        left.keys().eq(right.keys()),
        "Comparisons require exactly the same deals; no silent intersection"
    );
    let mut counts = BTreeMap::from([
        ("better", 0usize),
        ("equal", 0),
        ("worse", 0),
        ("unresolved_cap", 0),
        ("both_lose", 0),
        ("both_win", 0),
    ]);
    let mut pairs = Vec::new();
    for (key, a) in left {
        let b = right[&key];
        ensure!(
            a["base_utility"] == b["base_utility"] && a["base_capped"] == b["base_capped"],
            "Repeated house baseline differs on an identical deal"
        );
        let mut pair = a.clone();
        pair["base_utility"] = b["net_utility"].clone();
        pair["base_capped"] = b["net_capped"].clone();
        pairs.push(pair);
        if flag(a, "net_capped")? || flag(b, "net_capped")? {
            *counts.entry("unresolved_cap").or_default() += 1;
            continue;
        }
        let x = number(a, "net_utility")?;
        let y = number(b, "net_utility")?;
        *counts
            .entry(if x > y {
                "better"
            } else if x < y {
                "worse"
            } else {
                "equal"
            })
            .or_default() += 1;
        *counts.entry("both_lose").or_default() += usize::from(x == 0.0 && y == 0.0);
        // Utilities 0 and 1 are exact outcome labels, not approximate estimates.
        *counts.entry("both_win").or_default() +=
            usize::from(x.to_bits() == 1f64.to_bits() && y.to_bits() == 1f64.to_bits());
    }
    report(
        candidate,
        reference,
        cp,
        resamples,
        allow_thinking_difference,
        &counts,
        &pairs,
    )
}

fn check_protocol(
    candidate: &Value,
    reference: &Value,
    cp: &Value,
    rp: &Value,
    allow_thinking_difference: bool,
) -> Result<()> {
    ensure!(
        [
            "engine_sha256",
            "deck_manifest_sha256",
            "max_decisions",
            "fair_scouting",
            "holdout_tier",
            "generated"
        ]
        .iter()
        .all(|k| cp.get(k).is_some())
            && cp == rp,
        "Engine, deck content and evaluation limits must match"
    );
    for key in [
        "against",
        "baseline",
        "seats",
        "shape",
        "teams",
        "deck_split",
        "held_out_cards",
        "first_seed",
        "deal_offset",
        "deals",
        "policy_seed",
        "temperature",
        "thinking_steps",
        "policy_mode",
        "policy_contract",
    ] {
        if key == "thinking_steps" && allow_thinking_difference {
            ensure!(
                candidate.get(key).is_some() && reference.get(key).is_some(),
                "Evaluation protocol is missing: {key}"
            );
            continue;
        }
        if ["policy_mode", "policy_contract"].contains(&key)
            && candidate.get(key).is_none()
            && reference.get(key).is_none()
        {
            continue;
        }
        ensure!(
            candidate.get(key).is_some()
                && reference.get(key).is_some()
                && candidate[key] == reference[key],
            "Evaluation protocol differs or is missing: {key}"
        );
    }
    ensure!(
        candidate["no_baseline"] != true && reference["no_baseline"] != true,
        "House duplicates are required to audit matching outcomes"
    );
    Ok(())
}

fn report(
    candidate: &Value,
    reference: &Value,
    cp: &Value,
    resamples: usize,
    allow_thinking_difference: bool,
    counts: &BTreeMap<&str, usize>,
    pairs: &[Value],
) -> Result<Value> {
    let select = |v: &Value, keys: &[&str], nulls: bool| -> Value {
        Value::Object(
            keys.iter()
                .filter_map(|k| {
                    v.get(k)
                        .cloned()
                        .or_else(|| nulls.then_some(Value::Null))
                        .map(|x| (String::from(*k), x))
                })
                .collect(),
        )
    };
    Ok(
        json!({"candidate":candidate["model"],"reference":reference["model"],
        "thinking_steps":{"candidate":candidate["thinking_steps"],"reference":reference["thinking_steps"]},
        "different_thinking_budgets_allowed":allow_thinking_difference,
        "policy_execution":{"candidate":select(candidate,&["policy_mode","policy_contract","model_identity","search","catalog"],false),"reference":select(reference,&["policy_mode","policy_contract","model_identity","search","catalog"],false)},
        "provenance":cp,"outcomes":counts,
        "execution_quality":{"candidate":select(candidate,&["model_family","house_fallbacks","fallback_rate","refused","stalls","net_answer_caps"],true),"reference":select(reference,&["model_family","house_fallbacks","fallback_rate","refused","stalls","net_answer_caps"],true)},
        "paired_uncertainty":summarize(&json!({"deal_details":pairs}),resamples)?,
        "scope":"Same start/decks/focal seat/opponents/engine; policy substitution against fixed opponents, not direct head-to-head",
        "luck":"Keep every deal. Both losing does not prove unavoidability; later draws may diverge after different actions.",
        "outcome_counts":"better/equal/worse/unresolved_cap partition all cases; both_lose and both_win are subsets of equal.",
        "selection":"No automatic promotion; repeated selection on this suite needs an untouched final seed/deck holdout."}),
    )
}
type CaseKey = (u64, u64, Vec<String>, u64);
fn index(value: &Value) -> Result<BTreeMap<CaseKey, &Value>> {
    let mut result = BTreeMap::new();
    for row in value["deal_details"]
        .as_array()
        .context("missing deal_details")?
    {
        ensure!(
            result.insert(case_key(row, true)?, row).is_none(),
            "Duplicate table/seed/decks/focal-seat case"
        );
    }
    ensure!(
        Some(result.len() as u64) == value["deals"].as_u64(),
        "Evaluation is missing declared deals"
    );
    Ok(result)
}
