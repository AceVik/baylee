//! Batched CPU calibration aggregation after tensor prediction execution.
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
fn floats(value: &Value) -> Result<Vec<f64>> {
    value
        .as_array()
        .context("missing metric array")?
        .iter()
        .map(|v| {
            v.as_f64()
                .filter(|n| n.is_finite())
                .context("nonfinite calibration value")
        })
        .collect()
}
fn ids(value: &Value) -> Result<Vec<u64>> {
    value
        .as_array()
        .context("missing identity array")?
        .iter()
        .map(|v| v.as_u64().context("invalid trajectory identity"))
        .collect()
}
#[allow(clippy::cast_precision_loss)] // Statistical weights are inherently floating point.
fn weights(ids: &[u64]) -> Vec<f64> {
    let mut counts = BTreeMap::new();
    for id in ids {
        *counts.entry(id).or_insert(0usize) += 1;
    }
    ids.iter()
        .map(|id| 1.0 / (counts.len() as f64 * counts[id] as f64))
        .collect()
}
fn aggregate(prediction: &[f64], truth: &[f64], weights: &[f64], float32: bool) -> Value {
    let mass = weights.iter().sum::<f64>();
    let mut bins = [[0.0; 4]; 10];
    let mut mse = 0.0;
    let mut prediction_mean = 0.0;
    let mut outcome_mean = 0.0;
    for ((p, y), w) in prediction.iter().zip(truth).zip(weights) {
        let weight = w / mass;
        #[allow(clippy::cast_possible_truncation)]
        let error = if float32 {
            f64::from((*p as f32 - *y as f32).powi(2))
        } else {
            (p - y).powi(2)
        };
        #[allow(clippy::cast_possible_truncation)]
        let code = if float32 {
            f64::from((p.clamp(0.0, 1.0) as f32) * 10.0)
        } else {
            p.clamp(0.0, 1.0) * 10.0
        };
        let bin = &mut bins[(code as usize).min(9)];
        bin[0] += 1.0;
        bin[1] += weight;
        bin[2] += p * weight;
        bin[3] += y * weight;
        mse += error * weight;
        prediction_mean += p * weight;
        outcome_mean += y * weight;
    }
    let mut ece = 0.0;
    let mut calibration = Vec::new();
    for (i, [samples, mass, p, y]) in bins.into_iter().enumerate() {
        if mass > 0.0 {
            let prediction = p / mass;
            let outcome = y / mass;
            ece += mass * (prediction - outcome).abs();
            calibration.push(json!({"bin":i,"samples":samples as u64,"mass":mass,"prediction":prediction,"outcome":outcome}));
        }
    }
    json!({"mse":mse,"brier":mse,"prediction_mean":prediction_mean,"outcome_mean":outcome_mean,"ece":ece,"calibration":calibration})
}
fn summary(
    prediction: &[f64],
    truth: &[f64],
    trajectories: &[u64],
    float32: bool,
) -> Result<Value> {
    ensure!(
        prediction.len() == truth.len() && truth.len() == trajectories.len(),
        "calibration arrays differ in length"
    );
    ensure!(!truth.is_empty(), "empty calibration subset");
    ensure!(
        prediction.iter().chain(truth).all(|n| n.is_finite()),
        "nonfinite calibration value"
    );
    Ok(
        json!({"samples":truth.len(),"trajectories":trajectories.iter().collect::<BTreeSet<_>>().len(),"frame_weighted":aggregate(prediction,truth,&vec![1.0;truth.len()],float32),"trajectory_weighted":aggregate(prediction,truth,&weights(trajectories),float32)}),
    )
}
fn average(values: &[f64], ids: &[u64]) -> f64 {
    let weights = weights(ids);
    let mass = weights.iter().sum::<f64>();
    values.iter().zip(weights).map(|(v, w)| v * w).sum::<f64>() / mass
}
fn priors(rows: &Value) -> Result<Value> {
    let players = ids(&rows["players"])?;
    let truth = floats(&rows["outcome"])?;
    let trajectories = ids(&rows["trajectory"])?;
    ensure!(
        !truth.is_empty() && players.len() == truth.len() && trajectories.len() == truth.len(),
        "invalid training-prior arrays"
    );
    let mut priors = BTreeMap::new();
    for n in players.iter().collect::<BTreeSet<_>>() {
        let positions = players
            .iter()
            .enumerate()
            .filter(|(_, p)| *p == n)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        priors.insert(
            n.to_string(),
            average(
                &positions.iter().map(|i| truth[*i]).collect::<Vec<_>>(),
                &positions
                    .iter()
                    .map(|i| trajectories[*i])
                    .collect::<Vec<_>>(),
            ),
        );
    }
    Ok(json!({"prior":priors,"fallback":average(&truth,&trajectories)}))
}
fn evaluate(value: &Value) -> Result<Value> {
    let rows = &value["rows"];
    let actual = floats(&value["actual"])?;
    let truth = floats(&rows["outcome"])?;
    let trajectories = ids(&rows["trajectory"])?;
    let players = ids(&rows["players"])?;
    let turns = floats(&rows["turn_norm"])?;
    let count = actual.len();
    ensure!(
        truth.len() == count
            && trajectories.len() == count
            && players.len() == count
            && turns.len() == count,
        "calibration arrays differ in length"
    );
    let active = rows["active"].as_array().context("missing active seats")?;
    let allied = rows["allied"].as_array().context("missing allied seats")?;
    ensure!(
        active.len() == count && allied.len() == count,
        "seat arrays differ in length"
    );
    let mut baseline = Vec::new();
    let mut constant = Vec::new();
    let fallback = value["fallback"]
        .as_f64()
        .context("missing fallback prior")?;
    for i in 0..count {
        let a = active[i].as_array().context("invalid active seats")?;
        let b = allied[i].as_array().context("invalid allied seats")?;
        ensure!(a.len() == b.len(), "seat masks differ");
        let total = a.iter().filter(|v| **v == true).count();
        ensure!(total > 0, "no active seats");
        let own = a
            .iter()
            .zip(b)
            .filter(|(a, b)| a.as_bool() == Some(true) && b.as_bool() == Some(true))
            .count();
        baseline.push(f64::from(u32::try_from(own)?) / f64::from(u32::try_from(total)?));
        constant.push(
            value["priors"][players[i].to_string()]
                .as_f64()
                .unwrap_or(fallback),
        );
    }
    let mut outputs = BTreeMap::from([
        ("calibrated", actual),
        ("source", floats(&rows["source_value"])?),
        ("uniform_side", baseline),
        ("training_seatcount_prior", constant),
    ]);
    ensure!(
        outputs.values().all(|v| v.len() == count),
        "prediction arrays differ in length"
    );
    let groups = groups(rows, &players, &turns);
    let mut report = BTreeMap::new();
    for (name, positions) in groups {
        if positions.is_empty() {
            continue;
        }
        let mut models = BTreeMap::new();
        for (model, prediction) in &mut outputs {
            models.insert(
                *model,
                summary(
                    &positions.iter().map(|i| prediction[*i]).collect::<Vec<_>>(),
                    &positions.iter().map(|i| truth[*i]).collect::<Vec<_>>(),
                    &positions
                        .iter()
                        .map(|i| trajectories[*i])
                        .collect::<Vec<_>>(),
                    matches!(*model, "calibrated" | "source"),
                )?,
            );
        }
        report.insert(name, models);
    }
    Ok(serde_json::to_value(report)?)
}
/// Aggregate a complete metric array or final prediction pass in one native call.
///
/// # Errors
/// Rejects malformed dimensions, nonfinite scalars and overlapping game identities.
pub fn run(value: &Value) -> Result<Value> {
    match value["kind"].as_str() {
        Some("weights") => Ok(json!(weights(&ids(&value["trajectories"])?))),
        Some("summary") => summary(
            &floats(&value["prediction"])?,
            &floats(&value["truth"])?,
            &ids(&value["trajectories"])?,
            value["float32"] == true,
        ),
        Some("priors") => priors(&value["rows"]),
        Some("evaluate") => evaluate(value),
        Some("disjoint") => {
            let left = ids(&value["left"])?;
            let right = ids(&value["right"])?;
            let intersection = left
                .into_iter()
                .collect::<BTreeSet<_>>()
                .intersection(&right.into_iter().collect())
                .copied()
                .take(10)
                .collect::<Vec<_>>();
            ensure!(
                intersection.is_empty(),
                "Training/validation game leakage: {intersection:?}"
            );
            Ok(Value::Null)
        }
        _ => bail!("unsupported calibration aggregation"),
    }
}

fn groups(rows: &Value, players: &[u64], turns: &[f64]) -> BTreeMap<String, Vec<usize>> {
    let count = players.len();
    let mut groups = BTreeMap::from([("all".to_owned(), (0..count).collect::<Vec<_>>())]);
    for i in 0..count {
        let turn = turns[i];
        let name = if turn <= 0.25 {
            "turn_0_5"
        } else if turn <= 0.5 {
            "turn_6_10"
        } else if turn < 1.0 {
            "turn_11_19"
        } else {
            "turn_20plus"
        };
        for name in [
            format!("players_{}", players[i]),
            name.to_owned(),
            if rows["commander"][i] == true {
                "commander"
            } else {
                "noncommander"
            }
            .to_owned(),
            if rows["decision"][i] == true {
                "decision_samples"
            } else {
                "passive_samples"
            }
            .to_owned(),
        ] {
            groups.entry(name).or_default().push(i);
        }
        if rows["team"][i] == true {
            groups.entry("team".into()).or_default().push(i);
        }
    }
    groups
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equal_trajectory_mass_and_calibration_have_analytical_values() -> Result<()> {
        assert_eq!(
            weights(&[0, 1, 1, 1]),
            vec![0.5, 1.0 / 6.0, 1.0 / 6.0, 1.0 / 6.0]
        );
        let report = summary(
            &[0.0, 1.0, 1.0, 1.0],
            &[1.0, 1.0, 1.0, 1.0],
            &[0, 1, 1, 1],
            false,
        )?;
        assert_eq!(report["frame_weighted"]["mse"], 0.25);
        assert!((report["trajectory_weighted"]["mse"].as_f64().unwrap() - 0.5).abs() < 1e-14);
        assert_eq!(report["frame_weighted"]["calibration"][0]["outcome"], 1.0);
        assert!(run(&json!({"kind":"disjoint","left":[1,2],"right":[2,3]})).is_err());
        assert!(summary(&[f64::NAN], &[0.0], &[0], false).is_err());
        assert!(
            run(&json!({"kind":"summary","prediction":[null],"truth":[0.0],"trajectories":[0]}))
                .is_err()
        );
        Ok(())
    }
}
