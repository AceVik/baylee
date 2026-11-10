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
/// ECE and Brier of `before` then `after`, frame-weighted then trajectory-weighted.
fn paired_metrics(
    positions: &[usize],
    trajectories: &[u64],
    truth: &[f64],
    before: &[f64],
    after: &[f64],
    float32: bool,
) -> Result<[f64; 8]> {
    let picked_truth: Vec<f64> = positions.iter().map(|i| truth[*i]).collect();
    let mut out = [0.0; 8];
    for (w, weight) in [vec![1.0; positions.len()], weights(trajectories)]
        .iter()
        .enumerate()
    {
        for (m, prediction) in [before, after].iter().enumerate() {
            let picked: Vec<f64> = positions.iter().map(|i| prediction[*i]).collect();
            let report = aggregate(&picked, &picked_truth, weight, float32);
            out[w * 4 + m * 2] = report["ece"].as_f64().context("missing ECE")?;
            out[w * 4 + m * 2 + 1] = report["brier"].as_f64().context("missing Brier")?;
        }
    }
    Ok(out)
}
/// Game-cluster bootstrap of calibration before and after a refit.
///
/// Games are drawn with replacement from the evaluation's `SplitMix64` stream; a
/// game drawn twice contributes its trajectories twice, each copy counting as a
/// trajectory of its own. Each weighting reports the full-sample estimate and a
/// 95 % percentile interval of ECE and Brier for before, after, and the paired
/// difference after minus before (both read on the same draw).
fn bootstrap(value: &Value) -> Result<Value> {
    let truth = floats(&value["truth"])?;
    let before = floats(&value["before"])?;
    let after = floats(&value["after"])?;
    let trajectories = ids(&value["trajectories"])?;
    let games = ids(&value["games"])?;
    let float32 = value["float32"] == true;
    let resamples = usize::try_from(
        value["resamples"]
            .as_u64()
            .context("missing bootstrap resamples")?,
    )?;
    let seed = value["seed"].as_u64().context("missing bootstrap seed")?;
    let count = truth.len();
    ensure!(
        count > 0 && resamples > 0,
        "bootstrap needs samples and resamples"
    );
    ensure!(
        [before.len(), after.len(), trajectories.len(), games.len()]
            .iter()
            .all(|n| *n == count),
        "calibration arrays differ in length"
    );
    ensure!(
        truth
            .iter()
            .chain(&before)
            .chain(&after)
            .all(|n| n.is_finite()),
        "nonfinite calibration value"
    );
    let mut by_game: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for (i, game) in games.iter().enumerate() {
        by_game.entry(*game).or_default().push(i);
    }
    let clusters: Vec<&Vec<usize>> = by_game.values().collect();
    let span = trajectories.iter().max().copied().unwrap_or(0) + 1;
    let all: Vec<usize> = (0..count).collect();
    let estimate = paired_metrics(&all, &trajectories, &truth, &before, &after, float32)?;
    let mut state = seed;
    // Columns: the eight metrics, then after minus before for each metric pair.
    let mut draws = vec![Vec::with_capacity(resamples); 12];
    for _ in 0..resamples {
        let (mut positions, mut copies) = (Vec::with_capacity(count), Vec::with_capacity(count));
        for copy in 0..clusters.len() {
            let cluster = clusters[super::sample(&mut state, clusters.len())];
            let offset = u64::try_from(copy)? * span;
            positions.extend(cluster.iter().copied());
            copies.extend(cluster.iter().map(|i| offset + trajectories[*i]));
        }
        let m = paired_metrics(&positions, &copies, &truth, &before, &after, float32)?;
        for (slot, metric) in m.iter().enumerate() {
            draws[slot].push(*metric);
        }
        for w in 0..2 {
            for k in 0..2 {
                draws[8 + w * 2 + k].push(m[w * 4 + 2 + k] - m[w * 4 + k]);
            }
        }
    }
    for column in &mut draws {
        column.sort_by(f64::total_cmp);
    }
    let interval = |sorted: &[f64]| {
        json!([
            super::summary::quantile(sorted, 0.025),
            super::summary::quantile(sorted, 0.975)
        ])
    };
    let mut weightings = serde_json::Map::new();
    for (w, name) in ["frame_weighted", "trajectory_weighted"].iter().enumerate() {
        let mut metrics = serde_json::Map::new();
        for (k, metric) in ["ece", "brier"].iter().enumerate() {
            let (b, a) = (w * 4 + k, w * 4 + 2 + k);
            metrics.insert(
                (*metric).to_owned(),
                json!({
                    "before": {"estimate": estimate[b], "interval": interval(&draws[b])},
                    "after": {"estimate": estimate[a], "interval": interval(&draws[a])},
                    "after_minus_before": {"estimate": estimate[a] - estimate[b],
                                           "interval": interval(&draws[8 + w * 2 + k])}}),
            );
        }
        weightings.insert((*name).to_owned(), Value::Object(metrics));
    }
    Ok(
        json!({"games": clusters.len(), "samples": count, "resamples": resamples, "seed": seed,
              "level": 0.95, "rng": "splitmix64-rejection-v1",
              "cluster": "game; a repeated game's trajectories count as new ones",
              "frame_weighted": weightings["frame_weighted"],
              "trajectory_weighted": weightings["trajectory_weighted"]}),
    )
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
        Some("bootstrap") => bootstrap(value),
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
    #[test]
    fn game_bootstrap_is_seeded_paired_and_matches_the_full_sample() -> Result<()> {
        let truth = [1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        let before = [0.9, 0.8, 0.9, 0.7, 0.95, 0.85, 0.6, 0.8];
        let after = [0.6, 0.55, 0.4, 0.35, 0.65, 0.45, 0.5, 0.4];
        let request = |seed: u64, after: &[f64]| {
            json!({"kind":"bootstrap","truth":truth,"before":before,"after":after,
                   "trajectories":[0,0,1,1,2,2,3,3],"games":[10,10,11,11,12,12,13,13],
                   "resamples":400,"seed":seed,"float32":false})
        };
        let report = run(&request(7, &after))?;
        assert_eq!(report, run(&request(7, &after))?);
        assert_ne!(report, run(&request(8, &after))?);
        assert_eq!(
            (report["games"].clone(), report["samples"].clone()),
            (json!(4), json!(8))
        );
        let full = summary(&before, &truth, &[0, 0, 1, 1, 2, 2, 3, 3], false)?;
        for weighting in ["frame_weighted", "trajectory_weighted"] {
            let ece = &report[weighting]["ece"];
            assert_eq!(ece["before"]["estimate"], full[weighting]["ece"]);
            let [lo, hi] = [0, 1].map(|i| ece["before"]["interval"][i].as_f64().unwrap());
            assert!(lo <= hi && (0.0..=1.0).contains(&lo) && hi <= 1.0);
            let diff = ece["after_minus_before"]["estimate"].as_f64().unwrap();
            let expected = ece["after"]["estimate"].as_f64().unwrap()
                - ece["before"]["estimate"].as_f64().unwrap();
            assert!((diff - expected).abs() < 1e-15);
        }
        // An unchanged head differs from itself by exactly zero on every draw.
        let same = run(&request(7, &before))?;
        assert_eq!(
            same["trajectory_weighted"]["ece"]["after_minus_before"]["interval"],
            json!([0.0, 0.0])
        );
        assert!(
            run(
                &json!({"kind":"bootstrap","truth":[1.0],"before":[0.5],"after":[0.5],
                             "trajectories":[0],"games":[0,1],"resamples":10,"seed":1})
            )
            .is_err()
        );
        assert!(
            run(
                &json!({"kind":"bootstrap","truth":[1.0],"before":[0.5],"after":[0.5],
                             "trajectories":[0],"games":[0],"resamples":0,"seed":1})
            )
            .is_err()
        );
        Ok(())
    }
}
