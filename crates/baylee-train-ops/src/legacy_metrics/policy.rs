//! Historical policy/v3 report reductions; no tensor or optimizer operations.
use super::{floats, metrics, ratio, training};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fmt::Write as _};
fn ints(input: &Value, key: &str) -> Result<Vec<i64>> {
    serde_json::from_value(input[key].clone()).with_context(|| format!("policy vector {key}"))
}
fn mean(values: impl Iterator<Item = f64>, n: usize) -> Value {
    if n == 0 {
        Value::Null
    } else {
        json!(values.sum::<f64>() / ratio(n))
    }
}
pub(super) fn value_slices(input: &Value) -> Result<Value> {
    let p = floats(&input["prediction"])?;
    let step = ints(input, "step")?;
    let result = ints(input, "result")?;
    let team = ints(input, "team")?;
    let winners = ints(input, "winners")?;
    let seats = ints(input, "seats")?;
    let turn = ints(input, "turn")?;
    ensure!(
        [
            step.len(),
            result.len(),
            team.len(),
            winners.len(),
            seats.len(),
            turn.len()
        ]
        .into_iter()
        .all(|n| n == p.len()),
        "value slice vector lengths differ"
    );
    let y: Vec<_> = (0..p.len())
        .map(|i| {
            if seats[i] == 2 {
                f64::from(i32::try_from(result[i]).unwrap_or(-1)) / 2.0
            } else if winners[i] == 0 {
                0.5
            } else {
                f64::from(u8::from(winners[i] & team[i] != 0))
            }
        })
        .collect();
    let score = |indices: Vec<usize>| {
        metrics(
            &indices.iter().map(|&i| p[i]).collect::<Vec<_>>(),
            &indices.iter().map(|&i| y[i]).collect::<Vec<_>>(),
        )
    };
    let first: Vec<_> = (0..p.len()).filter(|&i| step[i] == 0).collect();
    let mut rows = json!({"all":score(first.clone())?});
    for seat in first.iter().map(|&i| seats[i]).collect::<BTreeSet<_>>() {
        let indices: Vec<_> = first
            .iter()
            .copied()
            .filter(|&i| seats[i] == seat)
            .collect();
        if indices.len() >= 100 {
            rows[format!("{seat} seats")] = score(indices)?;
        }
    }
    for (lo, hi) in [(1, 3), (4, 6), (7, 9), (10, 14), (15, 10_000)] {
        let indices: Vec<_> = first
            .iter()
            .copied()
            .filter(|&i| (lo..=hi).contains(&turn[i]))
            .collect();
        if indices.len() >= 100 {
            rows[format!(
                "turns {lo}-{}",
                if hi == 10_000 {
                    "+".into()
                } else {
                    hi.to_string()
                }
            )] = score(indices)?;
        }
    }
    Ok(rows)
}
pub(super) fn publish(input: &Value) -> Result<Value> {
    let v3 = input["v3"].as_bool().context("v3 report flag")?;
    let hits: Vec<bool> = serde_json::from_value(input["hits"].clone())?;
    let hits: Vec<f64> = hits.into_iter().map(|v| f64::from(u8::from(v))).collect();
    let usable: Vec<bool> = serde_json::from_value(input["usable"].clone())?;
    let kind = ints(input, "question")?;
    let profile = ints(input, "profile")?;
    let options = ints(input, "options")?;
    ensure!(
        [usable.len(), kind.len(), profile.len(), options.len()]
            .into_iter()
            .all(|n| n == hits.len()),
        "policy report vector lengths differ"
    );
    let selected: Vec<_> = (0..hits.len()).filter(|&i| usable[i]).collect();
    let groups = |keys: &[i64],
                  names: &Value,
                  minimum: usize,
                  nonnegative: bool|
     -> Result<(Value, Vec<String>)> {
        let mut result = json!({});
        let mut order = Vec::new();
        for key in keys.iter().copied().collect::<BTreeSet<_>>() {
            if nonnegative && key < 0 {
                continue;
            }
            let indices: Vec<_> = selected
                .iter()
                .copied()
                .filter(|&i| keys[i] == key)
                .collect();
            if indices.len() < minimum {
                continue;
            }
            let index = usize::try_from(key).context("negative report group")?;
            let label = names[index].as_str().context("report group label")?;
            result[label] = json!({"n":indices.len(),"agreement":mean(indices.iter().map(|&i|hits[i]),indices.len())});
            order.push(label.into());
        }
        Ok((result, order))
    };
    let (by_kind, kind_order) = groups(&kind, &input["kinds"], 100, false)?;
    let (by_profile, profile_order) =
        groups(&profile, &input["profiles"], if v3 { 100 } else { 1 }, true)?;
    ensure!(
        selected.iter().all(|&i| v3 || options[i] > 0),
        "policy options must be positive"
    );
    let mut report = input["report"].clone();
    report["held_out_steps"] = json!(selected.len());
    report["agreement"] = mean(selected.iter().map(|&i| hits[i]), selected.len());
    report["uniform_chance"] = mean(
        selected
            .iter()
            .map(|&i| 1.0 / ratio(usize::try_from(options[i].max(i64::from(v3))).unwrap_or(1))),
        selected.len(),
    );
    report["by_kind"] = by_kind;
    report["by_profile"] = by_profile;
    let text = render(&report, v3, kind_order, profile_order)?;
    training::publish_files(input, &report, &text)?;
    Ok(json!({"report":report,"markdown":text}))
}
fn render(
    report: &Value,
    v3: bool,
    kind_order: Vec<String>,
    profile_order: Vec<String>,
) -> Result<String> {
    let model = report["model"].as_str().context("policy model")?;
    let step = report["best_step"].as_u64().context("policy best step")?;
    let count = commas(
        report["held_out_steps"]
            .as_u64()
            .context("held out count")?,
    );
    let agreement = training::number(report, "agreement", 3);
    let chance = training::number(report, "uniform_chance", 3);
    let mut text = if v3 {
        format!(
            "# v3 report — {model}\n\nPolicy: top-1 agreement with the house on {count} held-out answer steps: **{agreement}** (uniform pick {chance}). Model: step {}.\n\n",
            commas(step)
        )
    } else {
        format!(
            "# Policy report — {model}\n\nTop-1 agreement with the house on {count} held-out answer steps: **{agreement}** (a uniform pick among the offered options: {chance}). Reported model: step {}.\n\n",
            commas(step)
        )
    };
    for (title, name, order) in [
        ("question", "by_kind", kind_order),
        ("house profile", "by_profile", profile_order),
    ] {
        writeln!(text, "| {title} | steps | agreement |\n|---|---|---|")?;
        for key in order {
            let row = &report[name][&key];
            writeln!(
                text,
                "| {key} | {} | {} |",
                commas(row["n"].as_u64().context("group count")?),
                training::number(row, "agreement", 3)
            )?;
        }
        text.push('\n');
    }
    if v3 {
        text.push_str("Value (the deciding seat's side wins):\n\n| slice | n | Brier | log loss | accuracy | ECE |\n|---|---|---|---|---|---|\n");
        let mut order: Vec<String> = report["value"]
            .as_object()
            .context("value slices")?
            .keys()
            .cloned()
            .collect();
        order.sort_by_key(|key| {
            if key == "all" {
                (0, 0)
            } else if key.ends_with(" seats") {
                (
                    1,
                    key.split(' ')
                        .next()
                        .and_then(|n| n.parse::<u64>().ok())
                        .unwrap_or(0),
                )
            } else {
                (
                    2,
                    key.trim_start_matches("turns ")
                        .split('-')
                        .next()
                        .and_then(|n| n.parse::<u64>().ok())
                        .unwrap_or(0),
                )
            }
        });
        for key in order {
            let row = &report["value"][&key];
            writeln!(
                text,
                "| {key} | {} | {} | {} | {} | {} |",
                commas(row["n"].as_u64().context("value count")?),
                training::number(row, "brier", 4),
                training::number(row, "logloss", 4),
                training::number(row, "accuracy", 3),
                training::number(row, "ece", 4)
            )?;
        }
    } else {
        text.pop();
    }
    Ok(text)
}
fn commas(n: u64) -> String {
    let digits = n.to_string();
    let mut result = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c);
    }
    result
}
