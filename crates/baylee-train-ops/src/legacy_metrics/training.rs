//! Grouping and report rendering for the retained Torch value training baseline.
use super::{floats, metrics};
use crate::files;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fmt::Write as _, io::Write as _, path::Path};
pub(super) fn grouped(input: &Value) -> Result<Value> {
    let keys: Vec<String> = serde_json::from_value(input["keys"].clone())?;
    let p = floats(&input["prediction"])?;
    let y = floats(&input["truth"])?;
    ensure!(
        keys.len() == p.len() && p.len() == y.len(),
        "group prediction vectors differ"
    );
    let order: Vec<String> = if input["order"].is_null() {
        keys.iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    } else {
        serde_json::from_value(input["order"].clone())?
    };
    let name = input["name"].as_str().context("group report name")?;
    let mut groups = json!({});
    for key in &order {
        let indices: Vec<_> = keys
            .iter()
            .enumerate()
            .filter(|(_, k)| *k == key)
            .map(|(i, _)| i)
            .collect();
        if indices.len() >= 200 {
            groups[key] = metrics(
                &indices.iter().map(|&i| p[i]).collect::<Vec<_>>(),
                &indices.iter().map(|&i| y[i]).collect::<Vec<_>>(),
            )?;
        }
    }
    Ok(json!({name:groups,format!("{name}_order"):order}))
}
pub(super) fn number(value: &Value, key: &str, digits: usize) -> String {
    value[key]
        .as_f64()
        .map_or_else(|| "n/a".into(), |v| format!("{v:.digits$}"))
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .with_context(|| format!("report field {key}"))
}
fn ordered(report: &Value, name: &str) -> Result<Vec<(String, Value)>> {
    let groups = report[name].as_object().context("grouped report map")?;
    let keys: Vec<String> = if report[format!("{name}_order")].is_array() {
        serde_json::from_value(report[format!("{name}_order")].clone())?
    } else {
        groups.keys().cloned().collect()
    };
    Ok(keys
        .into_iter()
        .filter_map(|key| groups.get(&key).map(|v| (key, v.clone())))
        .collect())
}
fn extremes(report: &Value, lines: &mut String) -> Result<()> {
    lines.push_str("\n## The extremes\n\nWhere a bar's credibility lives: positions rated under 10 % and over 90 %.\n\n| | rated under 10 %: positions, mean rating, won | rated over 90 %: positions, mean rating, won |\n|---|---|---|\n");
    let mut groups = vec![("all".to_owned(), report["overall"]["value net"].clone())];
    groups.extend(ordered(report, "by_progress")?);
    for (name, group) in groups {
        let rows = group["reliability"]
            .as_array()
            .context("reliability bins")?;
        let cell = |label: &str| -> String {
            rows.iter().find(|v| v["bin"] == label).map_or_else(
                || "—".into(),
                |row| {
                    format!(
                        "{} · {} · {}",
                        row["n"],
                        number(row, "predicted", 3),
                        number(row, "won", 3)
                    )
                },
            )
        };
        writeln!(
            lines,
            "| {name} | {} | {} |",
            cell("0.0-0.1"),
            cell("0.9-1.0")
        )?;
    }
    Ok(())
}
pub(super) fn markdown(report: &Value) -> Result<String> {
    let mut lines = format!(
        "# Value net report — {}\n\n**What it measures:** {}\n\n",
        text(report, "model")?,
        text(report, "measures")?
    );
    let hashes: Vec<String> = serde_json::from_value(report["working_hashes"].clone())?;
    writeln!(
        lines,
        "Dataset `{}`: {} training and {} held-out decisions ({} held-out games). Working-card sets: {}.\n",
        text(report, "dataset")?,
        report["train_samples"],
        report["held_out_samples"],
        report["held_out_games"],
        hashes.join(", ")
    )?;
    lines.push_str(
        "| | Brier ↓ | log loss ↓ | accuracy (decided) ↑ | ECE ↓ |\n|---|---|---|---|---|\n",
    );
    for name in [
        "coin",
        "logistic (life, hand, library, board, turn)",
        "value net",
    ] {
        let row = &report["overall"][name];
        writeln!(
            lines,
            "| {name} | {} | {} | {} | {} |",
            number(row, "brier", 4),
            number(row, "logloss", 4),
            number(row, "accuracy", 3),
            number(row, "ece", 4)
        )?;
    }
    extremes(report, &mut lines)?;
    if let Some(curve) = report["curve"].as_array().filter(|v| !v.is_empty()) {
        writeln!(
            lines,
            "\n## Training curve (held-out slice; reported model: step {})\n\n| step | train loss | held-out log loss | Brier | ECE |\n|---|---|---|---|---|",
            report["best_step"]
        )?;
        for row in curve {
            writeln!(
                lines,
                "| {} | {} | {} | {} | {} |",
                row["step"],
                number(row, "train_loss", 4),
                number(row, "logloss", 4),
                number(row, "brier", 4),
                number(row, "ece", 4)
            )?;
        }
    }
    lines.push_str("\n## Reliability (held out)\n\n| rated | positions | mean rating | won |\n|---|---|---|---|\n");
    for row in report["overall"]["value net"]["reliability"]
        .as_array()
        .context("overall reliability")?
    {
        writeln!(
            lines,
            "| {} | {} | {} | {} |",
            text(row, "bin")?,
            row["n"],
            number(row, "predicted", 3),
            number(row, "won", 3)
        )?;
    }
    for name in ["by_turn", "by_progress", "by_profile"] {
        writeln!(
            lines,
            "\n## {}\n\n| | positions | Brier | accuracy | ECE |\n|---|---|---|---|---|",
            name.replace('_', " ")
        )?;
        for (label, row) in ordered(report, name)? {
            writeln!(
                lines,
                "| {label} | {} | {} | {} | {} |",
                row["n"],
                number(&row, "brier", 4),
                number(&row, "accuracy", 3),
                number(&row, "ece", 4)
            )?;
        }
    }
    Ok(lines)
}
pub(super) fn publish(input: &Value) -> Result<Value> {
    let report = &input["report"];
    let markdown = markdown(report)?;
    publish_files(input, report, &markdown)?;
    Ok(json!({"markdown":markdown}))
}
pub(super) fn publish_files(input: &Value, report: &Value, markdown: &str) -> Result<()> {
    if let Some(root) = input["out"].as_str() {
        let root = Path::new(root);
        ensure!(
            root.is_dir(),
            "existing value checkpoint directory required"
        );
        files::atomic_json(&root.join("report.json"), report)?;
        let mut file = tempfile::NamedTempFile::new_in(root)?;
        file.write_all(markdown.as_bytes())?;
        file.as_file().sync_all()?;
        file.persist(root.join("report.md"))?;
        #[cfg(unix)]
        std::fs::File::open(root)?.sync_all()?;
    }
    Ok(())
}
