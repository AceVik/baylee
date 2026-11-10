//! Where two evaluations of the same deals part (`phase_a4 diverge`): per
//! deal, the first decision of the seat under test at which the two runs'
//! answers differ, read from `eval4 --decision-trace`
//! (`RECORDS/decisions.jsonl`), with the decision's kind and head and the
//! reference run's logit margin there.
//!
//! Both runs play the same deals with the same seeds at temperature 0, so up
//! to the first differing answer their games are the same game: what the
//! reference run saw at that decision is what the other run saw.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// The trace `eval4 --decision-trace` writes into its record directory.
pub const TRACE_FILE: &str = "decisions.jsonl";

/// One decision of the seat under test, as `eval4 --decision-trace` writes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceLine {
    /// The deal.
    pub i: u64,
    /// This decision's index among the seat under test's decisions of the deal.
    pub n: u32,
    /// The engine's decision sequence number.
    pub seq: u64,
    /// The game turn.
    pub turn: u32,
    /// The deciding seat.
    pub seat: u8,
    /// The question's `PendingKind4`.
    pub kind: String,
    /// The policy head that answered, when the net ran (`0` single choice,
    /// `1` subset, `2` pairs, `3` number).
    /// The head that answered it.
    pub head: Option<u8>,
    /// Who answered: `Net`, `Forced`, `House`, ...
    pub source: String,
    /// The answer.
    pub action: Value,
    /// The answer's logit margin (see [`menu_margin`]); absent without logits.
    pub margin: Option<f64>,
    /// `top2` or `closest_bit`.
    /// `top2` or `closest_bit`.
    pub margin_kind: Option<String>,
    /// How many candidates (or pairs) the head scored.
    pub options: Option<u32>,
}

/// A decision's logit margin from the net's menu (`netplay4` `menu_trace`):
/// for a single choice the best logit less the second best, for a subset or
/// pair head the closest call, the smallest `|logit|` (each bit is taken
/// above zero), for a number nothing. With the head and the options scored.
#[must_use]
pub fn menu_margin(menu: &Value) -> Option<(f64, &'static str, u8, usize)> {
    let head = u8::try_from(menu["head"].as_u64()?).ok()?;
    let logits = |key: &str| -> Vec<f64> {
        menu[key]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| row["logit"].as_f64())
                    .collect()
            })
            .unwrap_or_default()
    };
    match head {
        0 => {
            let mut scores = logits("options");
            scores.sort_by(|a, b| b.total_cmp(a));
            (scores.len() >= 2).then(|| (scores[0] - scores[1], "top2", head, scores.len()))
        }
        1 | 2 => {
            let scores = logits(if head == 1 { "options" } else { "pairs" });
            let closest = scores
                .iter()
                .map(|logit| logit.abs())
                .min_by(f64::total_cmp)?;
            Some((closest, "closest_bit", head, scores.len()))
        }
        _ => None,
    }
}

/// The margin buckets of the summary, upper bounds exclusive.
const BUCKETS: [(f64, &str); 6] = [
    (0.1, "<0.1"),
    (0.25, "0.1-0.25"),
    (0.5, "0.25-0.5"),
    (1.0, "0.5-1"),
    (2.0, "1-2"),
    (f64::INFINITY, ">=2"),
];

fn bucket(margin: Option<f64>) -> &'static str {
    margin.map_or("none", |margin| {
        BUCKETS
            .iter()
            .find(|(bound, _)| margin < *bound)
            .map_or(">=2", |(_, name)| name)
    })
}

/// A run's trace by deal, each deal's decisions in order.
///
/// # Errors
/// A missing or malformed trace, or a deal whose decisions are not numbered
/// `0, 1, 2, ...`.
pub fn read_trace(dir: &Path) -> Result<BTreeMap<u64, Vec<TraceLine>>> {
    let path = dir.join(TRACE_FILE);
    let file = File::open(&path).with_context(|| {
        format!(
            "{}: run eval4 with --record-out and --decision-trace",
            path.display()
        )
    })?;
    let mut deals: BTreeMap<u64, Vec<TraceLine>> = BTreeMap::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let row: TraceLine = serde_json::from_str(&line)
            .with_context(|| format!("{}: a trace line", path.display()))?;
        deals.entry(row.i).or_default().push(row);
    }
    for (deal, rows) in &mut deals {
        rows.sort_by_key(|row| row.n);
        ensure!(
            rows.iter()
                .enumerate()
                .all(|(at, row)| usize::try_from(row.n).is_ok_and(|n| n == at)),
            "{}: deal {deal}'s decisions are not numbered in order",
            path.display()
        );
    }
    Ok(deals)
}

/// Where one deal's two games part.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Divergence {
    /// The deal.
    pub deal: u64,
    /// The decision index; `None` when the two games are the same throughout.
    pub n: Option<u32>,
    /// The game turn of that decision.
    pub turn: Option<u32>,
    /// Its `PendingKind4`.
    pub kind: Option<String>,
    /// The head that answered it.
    pub head: Option<u8>,
    /// The reference run's margin at that decision.
    pub margin: Option<f64>,
    /// `top2` or `closest_bit`.
    pub margin_kind: Option<String>,
    /// The reference run's answer.
    pub reference: Option<Value>,
    /// The other run's answer.
    pub other: Option<Value>,
    /// One run decided no further in this deal.
    pub ended: bool,
}

/// The first decision of `deal` where `reference` and `other` answer differently.
#[must_use]
pub fn first_divergence(deal: u64, reference: &[TraceLine], other: &[TraceLine]) -> Divergence {
    let parted = (0..reference.len().max(other.len())).find(|&at| {
        match (reference.get(at), other.get(at)) {
            (Some(a), Some(b)) => a.action != b.action,
            _ => true,
        }
    });
    let Some(at) = parted else {
        return Divergence {
            deal,
            n: None,
            turn: None,
            kind: None,
            head: None,
            margin: None,
            margin_kind: None,
            reference: None,
            other: None,
            ended: false,
        };
    };
    let (a, b) = (reference.get(at), other.get(at));
    let seen = a.or(b);
    Divergence {
        deal,
        n: u32::try_from(at).ok(),
        turn: seen.map(|row| row.turn),
        kind: seen.map(|row| row.kind.clone()),
        head: seen.and_then(|row| row.head),
        margin: a.and_then(|row| row.margin),
        margin_kind: a.and_then(|row| row.margin_kind.clone()),
        reference: a.map(|row| row.action.clone()),
        other: b.map(|row| row.action.clone()),
        ended: a.is_none() || b.is_none(),
    }
}

/// What a record directory's run was: its deals' identity and its policy.
fn run_identity(dir: &Path) -> Result<Value> {
    let run: Value = serde_json::from_slice(&std::fs::read(dir.join("run.json"))?)
        .with_context(|| format!("{}/run.json", dir.display()))?;
    Ok(json!({
        "deal_offset": run["deal_offset"], "planned_deals": run["planned_deals"],
        "decks": run["decks"], "seats": run["seats"], "deck_split": run["deck_split"],
        "decks_manifest": run["decks_manifest"], "policy_mode": run["policy_mode"],
        "tactics_rules": run["tactics_rules"], "choice_guard": run["choice_guard"],
        "temperature": run["temperature"],
    }))
}

/// Each deal's seed, focal seat and decks, from `games.jsonl`.
fn games(dir: &Path) -> Result<BTreeMap<u64, Value>> {
    let text = std::fs::read_to_string(dir.join("games.jsonl"))
        .with_context(|| format!("{}/games.jsonl", dir.display()))?;
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let row: Value = serde_json::from_str(line)?;
            let deal = row["i"].as_u64().context("a game without its deal")?;
            Ok((
                deal,
                json!({"seed": row["seed"], "policy_seed": row["policy_seed"],
                    "focal_seat": row["focal_seat"]}),
            ))
        })
        .collect()
}

/// The report of two record directories of the same deals: `reference` (the
/// normal run) against `other` (`--memory-reset decision`, or any other arm).
///
/// # Errors
/// Runs of different deals, seeds or protocol, or a missing trace.
pub fn diverge(reference: &Path, other: &Path) -> Result<Value> {
    let (a, b) = (run_identity(reference)?, run_identity(other)?);
    ensure!(
        a == b,
        "the two runs differ in deals or protocol:\n{a}\n{b}"
    );
    let (games_a, games_b) = (games(reference)?, games(other)?);
    ensure!(
        games_a == games_b,
        "the two runs did not play the same deals with the same seeds"
    );
    let (trace_a, trace_b) = (read_trace(reference)?, read_trace(other)?);
    let empty = Vec::new();
    let mut rows = Vec::new();
    let mut by_kind: BTreeMap<String, BTreeMap<&str, u64>> = BTreeMap::new();
    let mut by_margin: BTreeMap<&str, u64> = BTreeMap::new();
    let mut by_head: BTreeMap<String, u64> = BTreeMap::new();
    let mut identical = 0_u64;
    for &deal in games_a.keys() {
        let divergence = first_divergence(
            deal,
            trace_a.get(&deal).unwrap_or(&empty),
            trace_b.get(&deal).unwrap_or(&empty),
        );
        if let Some(kind) = &divergence.kind {
            let bucket = bucket(divergence.margin);
            *by_kind
                .entry(kind.clone())
                .or_default()
                .entry(bucket)
                .or_default() += 1;
            *by_margin.entry(bucket).or_default() += 1;
            *by_head
                .entry(
                    divergence
                        .head
                        .map_or_else(|| "none".to_owned(), |h| h.to_string()),
                )
                .or_default() += 1;
        } else {
            identical += 1;
        }
        rows.push(divergence);
    }
    let diverged: Vec<u32> = rows.iter().filter_map(|row| row.n).collect();
    let mut sorted = diverged.clone();
    sorted.sort_unstable();
    let median = sorted.get(sorted.len() / 2).copied();
    Ok(json!({
        "reference": reference, "other": other,
        "reference_memory_reset": memory_reset(reference)?,
        "other_memory_reset": memory_reset(other)?,
        "deals": games_a.len(), "identical": identical, "diverged": diverged.len(),
        "median_first_divergence": median,
        "margin_buckets": BUCKETS.iter().map(|(_, name)| *name).chain(["none"]).collect::<Vec<_>>(),
        "by_margin": by_margin, "by_kind": by_kind, "by_head": by_head,
        "first_divergence": rows,
    }))
}

fn memory_reset(dir: &Path) -> Result<Value> {
    let run: Value = serde_json::from_slice(&std::fs::read(dir.join("run.json"))?)?;
    Ok(run.get("memory_reset").cloned().unwrap_or(Value::Null))
}

/// The summary as a Markdown table: kinds by margin bucket.
#[must_use]
pub fn table(report: &Value) -> String {
    let buckets: Vec<&str> = BUCKETS
        .iter()
        .map(|(_, name)| *name)
        .chain(["none"])
        .collect();
    let mut out = format!(
        "deals {}, identical {}, diverged {}, median first divergence {}\n\n| kind | {} | all |\n|---|{}---|\n",
        report["deals"],
        report["identical"],
        report["diverged"],
        report["median_first_divergence"],
        buckets.join(" | "),
        "---|".repeat(buckets.len()),
    );
    if let Some(kinds) = report["by_kind"].as_object() {
        for (kind, counts) in kinds {
            let cells: Vec<u64> = buckets
                .iter()
                .map(|name| counts[*name].as_u64().unwrap_or(0))
                .collect();
            let all: u64 = cells.iter().sum();
            let cells: Vec<String> = cells.iter().map(u64::to_string).collect();
            let _ = writeln!(out, "| {kind} | {} | {all} |", cells.join(" | "));
        }
    }
    out
}

#[cfg(test)]
mod tests;
