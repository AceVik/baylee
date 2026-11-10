//! An arm against its reference on one build, where the arm may differ from
//! the reference in exactly what `paired_eval4` refuses: temperature, policy
//! seed, thinking steps, the dev flags.
//!
//! An arm may hold several runs of the same deals, one per policy seed. Each
//! deal's score is then the mean over the seeds, and the bootstrap over tables
//! sees that mean, so sampling noise stays inside the interval: a table's
//! difference carries the noise of its games, and more seeds only shrink it.
// Counts are bounded by resident JSON input sizes, well below f64's exact 2^53 range.
#![allow(clippy::cast_precision_loss)]
use super::{case_key, flag, number, summarize};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::collections::BTreeMap;

type Key = (u64, u64, Vec<String>, u64);

/// The keys of the protocol that must match, whatever the arm changes.
const SAME: [&str; 8] = [
    "against",
    "seats",
    "shape",
    "deck_split",
    "first_seed",
    "deal_offset",
    "model",
    "evaluator_revision",
];

fn deals(value: &Value) -> Result<BTreeMap<Key, &Value>> {
    let mut rows = BTreeMap::new();
    for row in value["deal_details"]
        .as_array()
        .context("missing deal_details")?
    {
        ensure!(
            rows.insert(case_key(row, true)?, row).is_none(),
            "Duplicate table/seed/decks/focal-seat case"
        );
    }
    Ok(rows)
}

/// `candidates` against `reference`. The reference may hold more deals than
/// the candidates; the comparison is on the candidates' deals, and each
/// candidate must hold the same ones.
///
/// # Errors
/// Rejects different protocols, builds or deal sets, and dirty builds.
pub fn compare_arm(candidates: &[Value], reference: &Value, resamples: usize) -> Result<Value> {
    // One `--model` can mean two exports; reports older than the key have none.
    for run in candidates {
        ensure!(
            run["precision"] == reference["precision"],
            "Protocol differs: precision"
        );
    }
    compare(candidates, reference, resamples, &SAME)
}

/// `compare_arm` between exports of one net: the same protocol but for the
/// model, as FP16 against FP32, or one trunk under two value heads.
///
/// # Errors
/// As `compare_arm`.
pub fn compare_exports(candidates: &[Value], reference: &Value, resamples: usize) -> Result<Value> {
    let protocol: Vec<&str> = SAME.into_iter().filter(|key| *key != "model").collect();
    compare(candidates, reference, resamples, &protocol)
}

/// `compare_arm` between two builds of the evaluator, the change between
/// them under test (a view the net reads, a rule): the same protocol and
/// model, only `evaluator_revision` may differ. Both builds must still be
/// clean commits.
///
/// # Errors
/// As `compare_arm`, and when the builds do not differ.
pub fn compare_builds(candidates: &[Value], reference: &Value, resamples: usize) -> Result<Value> {
    for run in candidates {
        ensure!(
            run["precision"] == reference["precision"],
            "Protocol differs: precision"
        );
        ensure!(
            run["evaluator_revision"] != reference["evaluator_revision"],
            "One build on both sides: pair it as an arm"
        );
    }
    let protocol: Vec<&str> = SAME
        .into_iter()
        .filter(|key| *key != "evaluator_revision")
        .collect();
    compare(candidates, reference, resamples, &protocol)
}

fn compare(
    candidates: &[Value],
    reference: &Value,
    resamples: usize,
    protocol: &[&str],
) -> Result<Value> {
    ensure!(!candidates.is_empty(), "An arm needs at least one run");
    let baseline = deals(reference)?;
    let first = deals(&candidates[0])?;
    for run in candidates {
        for &key in protocol {
            ensure!(
                run.get(key).is_some() && run[key] == reference[key],
                "Protocol differs or is missing: {key}"
            );
        }
        ensure!(
            run["evaluator_dirty"] == false && reference["evaluator_dirty"] == false,
            "A dirty build is not a frozen build"
        );
        ensure!(
            deals(run)?.keys().eq(first.keys()),
            "Every run of an arm must hold the same deals"
        );
    }
    ensure!(
        first.keys().all(|key| baseline.contains_key(key)),
        "The reference lacks a deal of the arm"
    );
    let runs: Vec<_> = candidates.iter().map(deals).collect::<Result<_>>()?;
    let (mut better, mut worse, mut equal) = (0usize, 0usize, 0usize);
    let (mut arm_wins, mut reference_wins) = (0.0_f64, 0.0_f64);
    let mut rows = Vec::with_capacity(first.len());
    let seeds = runs.len() as f64;
    for (key, row) in &first {
        let reference_row = baseline[key];
        let reference_score = number(reference_row, "net_utility")?;
        let mut score = 0.0;
        let mut capped = false;
        for run in &runs {
            score += number(run[key], "net_utility")?;
            capped |= flag(run[key], "net_capped")?;
        }
        score /= seeds;
        arm_wins += score;
        reference_wins += reference_score;
        if score > reference_score {
            better += 1;
        } else if score < reference_score {
            worse += 1;
        } else {
            equal += 1;
        }
        rows.push(
            json!({"table": row["table"], "seed": row["seed"], "decks": row["decks"],
            "net_utility": score, "base_utility": reference_score,
            "net_capped": capped, "base_capped": flag(reference_row, "net_capped")?}),
        );
    }
    let per_seed: Vec<Value> = candidates
        .iter()
        .map(|run| json!({"policy_seed": run["policy_seed"], "net_wins": run["net_wins"]}))
        .collect();
    Ok(json!({
        "deals": first.len(),
        "seeds": candidates.len(),
        "arm_wins_mean": arm_wins,
        "reference_wins": reference_wins,
        "better": better, "equal": equal, "worse": worse,
        "per_seed": per_seed,
        "paired": summarize(&json!({"deal_details": rows}), resamples)?,
        "rows": rows,
        "execution": candidates.iter().map(|run| json!({
            "refused": run["refused"], "stalls": run["stalls"],
            "house_fallbacks": run["house_fallbacks"], "net_answer_caps": run["net_answer_caps"],
            "elapsed_secs": run["elapsed_secs"], "menu": run["menu"],
        })).collect::<Vec<_>>(),
    }))
}

/// Several reports of one arm on different deals as one: dev and fresh are
/// independent tables, so the pooled interval is one bootstrap over all of them.
///
/// # Errors
/// Rejects an empty list or reports without their rows.
pub fn pool_arms(reports: &[Value], resamples: usize) -> Result<Value> {
    ensure!(!reports.is_empty(), "Nothing to pool");
    let mut rows = Vec::new();
    let total = |key: &str| -> f64 { reports.iter().filter_map(|r| r[key].as_f64()).sum() };
    for report in reports {
        rows.extend(
            report["rows"]
                .as_array()
                .context("missing rows")?
                .iter()
                .cloned(),
        );
    }
    Ok(json!({
        "deals": rows.len(), "seeds": reports[0]["seeds"],
        "arm_wins_mean": total("arm_wins_mean"), "reference_wins": total("reference_wins"),
        "better": total("better"), "equal": total("equal"), "worse": total("worse"),
        "paired": summarize(&json!({"deal_details": rows}), resamples)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_and_fresh_pool_into_one_interval() {
        let reference = run(1, &[0.0, 1.0, 0.0, 1.0]);
        let mut other_reference = run(1, &[0.0, 0.0, 1.0, 1.0]);
        let mut other_arm = run(2, &[1.0, 1.0, 1.0, 1.0]);
        for value in [&mut other_reference, &mut other_arm] {
            value["first_seed"] = json!(9);
            for row in value["deal_details"].as_array_mut().unwrap() {
                row["seed"] = json!(row["seed"].as_u64().unwrap() + 100);
            }
        }
        let one = compare_arm(&[run(2, &[1.0, 1.0, 0.0, 1.0])], &reference, 100).unwrap();
        let two = compare_arm(&[other_arm], &other_reference, 100).unwrap();
        let pooled = pool_arms(&[one, two], 100).unwrap();
        assert_eq!(pooled["deals"], 8);
        assert_eq!(pooled["better"], 3.0);
        assert_eq!(pooled["paired"]["paired_utility_difference"], 0.375);
    }

    fn run(seed: u64, scores: &[f64]) -> Value {
        let details: Vec<Value> = scores
            .iter()
            .enumerate()
            .map(|(deal, score)| {
                json!({"deal": deal, "table": deal / 2, "seed": 7 + deal / 2,
                    "focal_seat": deal % 2, "decks": ["a", "b"],
                    "net_utility": score, "net_capped": false})
            })
            .collect();
        json!({"against": "sharp", "seats": 2, "shape": "constructed", "deck_split": "test",
            "first_seed": 7, "deal_offset": 0, "model": "m", "evaluator_revision": "r",
            "evaluator_dirty": false, "policy_seed": seed, "net_wins": 0,
            "deal_details": details})
    }

    #[test]
    fn seeds_average_inside_a_deal() {
        let reference = run(1, &[0.0, 1.0, 0.0, 1.0]);
        let arm = [run(2, &[1.0, 1.0, 0.0, 1.0]), run(3, &[0.0, 1.0, 0.0, 1.0])];
        let report = compare_arm(&arm, &reference, 100).unwrap();
        assert_eq!(report["better"], 1);
        assert_eq!(report["worse"], 0);
        assert_eq!(report["arm_wins_mean"], 2.5);
        assert_eq!(report["paired"]["paired_utility_difference"], 0.125);
    }

    #[test]
    fn another_build_is_refused() {
        let reference = run(1, &[0.0, 1.0]);
        let mut arm = run(2, &[1.0, 1.0]);
        arm["evaluator_revision"] = json!("other");
        assert!(compare_arm(&[arm], &reference, 10).is_err());
    }

    #[test]
    fn two_builds_pair_only_as_builds() {
        let reference = run(1, &[0.0, 1.0]);
        let mut arm = run(1, &[1.0, 1.0]);
        arm["evaluator_revision"] = json!("other");
        let report = compare_builds(&[arm.clone()], &reference, 10).unwrap();
        assert_eq!(report["better"], 1);
        assert!(compare_builds(std::slice::from_ref(&reference), &reference, 10).is_err());
        arm["model"] = json!("another net");
        assert!(compare_builds(&[arm.clone()], &reference, 10).is_err());
        arm["model"] = json!("m");
        arm["evaluator_dirty"] = json!(true);
        assert!(compare_builds(&[arm], &reference, 10).is_err());
    }

    #[test]
    fn a_reference_may_hold_more_deals() {
        let reference = run(1, &[0.0, 1.0, 0.0, 1.0]);
        let arm = run(2, &[1.0, 1.0]);
        let report = compare_arm(&[arm], &reference, 10).unwrap();
        assert_eq!(report["deals"], 2);
        assert_eq!(report["reference_wins"], 1.0);
    }
}
