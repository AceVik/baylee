//! Batched host aggregation of on-device diagnostic sufficient statistics.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
const FIELDS: &[&str] = &[
    "frames",
    "decisions",
    "zero_weight_decisions",
    "forced_source_decisions",
    "single_choices",
    "single_options",
    "single_nll_sum",
    "single_correct",
    "pass_targets",
    "pass_correct",
    "pass_nll_sum",
    "active_targets",
    "active_correct",
    "active_nll_sum",
    "subset_choices",
    "subset_free_entries",
    "subset_bce_sum",
    "subset_decision_bce_sum",
    "subset_exact_free_set",
    "pair_choices",
    "pair_free_entries",
    "pair_bce_sum",
    "pair_decision_bce_sum",
    "pair_exact_free_set",
    "scalar_decisions",
    "scalar_absolute_error_sum",
    "scalar_squared_error_sum",
    "critic_squared_error_sum",
    "critic_prediction_sum",
    "outcome_sum",
    "pass_target_predicted_pass",
    "pass_target_predicted_active",
    "active_target_predicted_pass",
    "active_target_predicted_active",
    "pass_target_predicted_other_pass",
];
type Groups = BTreeMap<String, BTreeMap<String, Vec<f64>>>;
#[derive(Default, Serialize, Deserialize)]
struct State {
    total: Vec<f64>,
    groups: Groups,
    games: BTreeMap<String, Value>,
}
fn summary(values: &[f64]) -> Result<Value> {
    ensure!(
        values.len() == FIELDS.len() && values.iter().all(|n| n.is_finite()),
        "Nonfinite or malformed diagnostic statistics"
    );
    let sums = FIELDS
        .iter()
        .copied()
        .zip(values.iter().copied())
        .collect::<BTreeMap<_, _>>();
    let ratio = |a: &str, b: &str| {
        if sums[b] == 0.0 {
            Value::Null
        } else {
            json!(sums[a] / sums[b])
        }
    };
    let mut metrics = BTreeMap::new();
    for (name, num, den) in [
        ("single_nll", "single_nll_sum", "single_choices"),
        ("single_accuracy", "single_correct", "single_choices"),
        ("pass_target_accuracy", "pass_correct", "pass_targets"),
        ("active_target_accuracy", "active_correct", "active_targets"),
        ("pass_target_nll", "pass_nll_sum", "pass_targets"),
        ("active_target_nll", "active_nll_sum", "active_targets"),
        (
            "scalar_mae",
            "scalar_absolute_error_sum",
            "scalar_decisions",
        ),
        ("scalar_mse", "scalar_squared_error_sum", "scalar_decisions"),
        ("critic_mse", "critic_squared_error_sum", "frames"),
    ] {
        metrics.insert(name.to_owned(), ratio(num, den));
    }
    for target in ["pass", "active"] {
        for predicted in ["pass", "active"] {
            metrics.insert(
                format!("{target}_target_predicted_{predicted}_rate"),
                ratio(
                    &format!("{target}_target_predicted_{predicted}"),
                    &format!("{target}_targets"),
                ),
            );
        }
    }
    metrics.insert(
        "pass_target_predicted_other_pass_rate".into(),
        ratio("pass_target_predicted_other_pass", "pass_targets"),
    );
    metrics.insert(
        "predicted_pass_rate".into(),
        if sums["single_choices"] == 0.0 {
            Value::Null
        } else {
            json!(
                (sums["pass_target_predicted_pass"] + sums["active_target_predicted_pass"])
                    / sums["single_choices"]
            )
        },
    );
    for head in ["subset", "pair"] {
        for (label, num, den) in [
            ("free_entry_bce", "bce_sum", "free_entries"),
            ("mean_decision_bce", "decision_bce_sum", "choices"),
            (
                "threshold_exact_free_set_accuracy",
                "exact_free_set",
                "choices",
            ),
        ] {
            metrics.insert(
                format!("{head}_{label}"),
                ratio(&format!("{head}_{num}"), &format!("{head}_{den}")),
            );
        }
    }
    Ok(json!({"sums":sums,"metrics":metrics}))
}
fn accumulate(target: &mut [f64], values: &[f64]) {
    for (a, b) in target.iter_mut().zip(values) {
        *a += b;
    }
}
fn statistic_row(row: &Value) -> Result<Vec<f64>> {
    let row = row
        .as_array()
        .context("invalid diagnostic row")?
        .iter()
        .map(|v| {
            v.as_f64()
                .filter(|v| v.is_finite())
                .context("Nonfinite diagnostic statistics")
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(row.len() == FIELDS.len(), "invalid diagnostic row width");
    Ok(row)
}
fn add(state: &mut State, value: &Value) -> Result<()> {
    if state.total.is_empty() {
        state.total = vec![0.0; FIELDS.len()];
    }
    for dimension in [
        "head",
        "pending_kind",
        "format",
        "game",
        "head_pending_format",
    ] {
        state.groups.entry(dimension.into()).or_default();
    }
    let rows = value["stats"]
        .as_array()
        .context("missing diagnostic batch")?;
    let arrays = &value["arrays"];
    ensure!(
        state.total.len() == FIELDS.len(),
        "invalid diagnostic state width"
    );
    for key in [
        "diag_game",
        "diag_seat",
        "diag_pending",
        "head",
        "step_valid",
    ] {
        ensure!(
            arrays[key]
                .as_array()
                .is_some_and(|values| values.len() == rows.len()),
            "diagnostic arrays differ in length"
        );
    }
    let provenance = &value["provenance"];
    let namespace = provenance["namespace"]
        .as_str()
        .context("missing conversion namespace")?;
    for (i, row) in rows.iter().enumerate() {
        if arrays["step_valid"][i] != true {
            continue;
        }
        let row = statistic_row(row)?;
        let game = arrays["diag_game"][i]
            .as_u64()
            .context("invalid diagnostic game")?;
        let seat = arrays["diag_seat"][i]
            .as_u64()
            .context("invalid diagnostic seat")?;
        let head = arrays["head"][i]
            .as_i64()
            .context("invalid head")?
            .to_string();
        let pending = arrays["diag_pending"][i]
            .as_i64()
            .context("invalid pending kind")?
            .to_string();
        let key = format!("{namespace}:{game}");
        let game = game.to_string();
        let format = provenance["formats"][&game].as_str().unwrap_or("unknown");
        let metadata=state.games.entry(key.clone()).or_insert_with(||json!({"namespace":namespace,"game_id":game.parse::<u64>().expect("converted integer"),"format":format,"seats":[],"source":provenance["sources"][&game]}));
        ensure!(
            metadata["format"] == format,
            "Game has conflicting format identities"
        );
        let mut seats = metadata["seats"]
            .as_array()
            .context("invalid game seats")?
            .iter()
            .map(|v| v.as_u64().context("invalid seat"))
            .collect::<Result<BTreeSet<_>>>()?;
        seats.insert(seat);
        metadata["seats"] = json!(seats);
        accumulate(&mut state.total, &row);
        for (dimension, key) in [
            ("head", head.clone()),
            ("pending_kind", pending.clone()),
            ("format", format.to_owned()),
            ("game", key),
            (
                "head_pending_format",
                serde_json::to_string(&[head, pending, format.to_owned()])?,
            ),
        ] {
            let target = state
                .groups
                .entry(dimension.into())
                .or_default()
                .entry(key)
                .or_insert_with(|| vec![0.0; FIELDS.len()]);
            accumulate(target, &row);
        }
    }
    Ok(())
}
fn report(state: &State) -> Result<Value> {
    let mut result = json!({"global_metrics":summary(&state.total)?});
    for (dimension, name) in [
        ("head", "by_head"),
        ("pending_kind", "by_pending_kind"),
        ("format", "by_format"),
    ] {
        let values = state
            .groups
            .get(dimension)
            .context("missing diagnostic group")?
            .iter()
            .map(|(k, v)| Ok((k, summary(v)?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        result[name] = json!(values);
    }
    let mut joint = Vec::new();
    for (key, value) in state
        .groups
        .get("head_pending_format")
        .context("missing joint group")?
    {
        let key: Vec<String> = serde_json::from_str(key)?;
        let mut entry = summary(value)?;
        entry["head"] = json!(key[0].parse::<i64>()?);
        entry["pending_kind"] = json!(key[1].parse::<i64>()?);
        entry["format"] = json!(key[2]);
        joint.push(entry);
    }
    result["by_head_pending_format"] = json!(joint);
    let mut games = Vec::new();
    for (key, value) in state.groups.get("game").context("missing game group")? {
        let mut entry = state.games[key].clone();
        entry
            .as_object_mut()
            .context("invalid game metadata")?
            .extend(
                summary(value)?
                    .as_object()
                    .context("invalid summary")?
                    .clone(),
            );
        games.push(entry);
    }
    result["per_game"] = json!(games);
    Ok(result)
}
/// Merge one complete unroll batch or format an accumulated report.
///
/// # Errors
/// Rejects schema mismatches, nonfinite valid rows and conflicting game formats.
pub fn run(value: &Value) -> Result<Value> {
    ensure!(
        value["fields"] == json!(FIELDS),
        "Diagnostic sufficient-statistic schema differs"
    );
    if value["kind"] == "summary" {
        return summary(&serde_json::from_value::<Vec<f64>>(
            value["values"].clone(),
        )?);
    }
    let mut state: State = serde_json::from_value(value["state"].clone())?;
    match value["kind"].as_str() {
        Some("add") => {
            add(&mut state, value)?;
            Ok(serde_json::to_value(state)?)
        }
        Some("report") => report(&state),
        _ => bail!("unsupported diagnostic aggregation"),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn batched_statistics_cluster_games_and_leave_empty_ratios_null() -> Result<()> {
        let mut row = vec![0.0; FIELDS.len()];
        for key in [
            "frames",
            "decisions",
            "single_choices",
            "single_correct",
            "pass_targets",
            "pass_correct",
        ] {
            row[FIELDS.iter().position(|name| *name == key).unwrap()] = 1.0;
        }
        let batch = json!({"arrays":{"diag_game":[7,7,999],"diag_seat":[0,1,0],"diag_pending":[3,3,0],"head":[0,0,0],"step_valid":[true,true,false]},"stats":[row,row,null],"provenance":{"namespace":"conversion","formats":{"7":"duels"},"sources":{}}});
        let mut state = State::default();
        add(&mut state, &batch)?;
        let result = report(&state)?;
        assert_eq!(result["global_metrics"]["sums"]["frames"], 2.0);
        assert_eq!(result["global_metrics"]["metrics"]["single_accuracy"], 1.0);
        assert!(result["global_metrics"]["metrics"]["pair_free_entry_bce"].is_null());
        assert_eq!(result["per_game"][0]["seats"], json!([0, 1]));
        assert_eq!(result["per_game"].as_array().unwrap().len(), 1);
        let mut changed = batch.clone();
        changed["provenance"]["formats"]["7"] = json!("different");
        assert!(add(&mut state, &changed).is_err());
        Ok(())
    }
}
