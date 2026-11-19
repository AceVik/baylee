//! Whether two `eval4 --search` runs of the same deals searched alike: the
//! same games, the same searched decisions with the same candidates and
//! choices, and how far their worlds' scores sit apart. What a change to
//! how a search runs (servers, batches, threads) must keep: its answers,
//! with scores equal up to floating-point noise.
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{Value, json};

/// One game of a run's `games.jsonl`, keyed by its deal.
fn games(dir: &Path) -> Result<BTreeMap<u64, Value>> {
    let path = dir.join("games.jsonl");
    let file = std::fs::File::open(&path).with_context(|| format!("opening {}", path.display()))?;
    let mut games = BTreeMap::new();
    for (n, line) in BufReader::new(file).lines().enumerate() {
        let line = line.with_context(|| format!("reading {}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        let game: Value = serde_json::from_str(&line)
            .with_context(|| format!("{} line {}", path.display(), n + 1))?;
        let deal = game["i"]
            .as_u64()
            .with_context(|| format!("{} line {} has no deal `i`", path.display(), n + 1))?;
        anyhow::ensure!(
            games.insert(deal, game).is_none(),
            "{} holds deal {deal} twice",
            path.display()
        );
    }
    Ok(games)
}

/// The largest absolute difference between two arrays of numbers, nested
/// alike; `None` when their shapes differ.
fn largest_gap(a: &Value, b: &Value) -> Option<f64> {
    match (a, b) {
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => a
            .iter()
            .zip(b)
            .try_fold(0.0_f64, |worst, (a, b)| Some(worst.max(largest_gap(a, b)?))),
        (Value::Number(a), Value::Number(b)) => Some((a.as_f64()? - b.as_f64()?).abs()),
        (Value::Null, Value::Null) => Some(0.0),
        _ => None,
    }
}

/// Where two runs first part, for the report.
fn parting(deal: u64, event: Option<&Value>, why: &str) -> Value {
    json!({
        "deal": deal,
        "decision_seq": event.and_then(|e| e.get("decision_seq")).cloned(),
        "why": why,
    })
}

/// Compares the record directories `a` and `b` of two runs of the same deals.
///
/// A game counts as the same when its record's digest is; a searched
/// decision as the same when its position (`decision_seq`), candidates,
/// choice and played action are. The worlds' scores and the means of the
/// decisions that are the same give the largest gaps.
///
/// # Errors
/// When a `games.jsonl` cannot be read.
pub fn search_parity(a: &Path, b: &Path) -> Result<Value> {
    let (a_games, b_games) = (games(a)?, games(b)?);
    let mut report = Parity::default();
    for (deal, game_a) in &a_games {
        let Some(game_b) = b_games.get(deal) else {
            report.unpaired += 1;
            continue;
        };
        report.games += 1;
        if game_a["record_sha256"] == game_b["record_sha256"] {
            report.same_records += 1;
        } else if report.first_parting.is_none() {
            report.first_parting = Some(parting(*deal, None, "record"));
        }
        if game_a["outcome"] == game_b["outcome"] {
            report.same_outcomes += 1;
        }
        report.compare_events(*deal, game_a, game_b);
    }
    report.unpaired += b_games.keys().filter(|d| !a_games.contains_key(d)).count();
    Ok(report.into_json(a, b))
}

/// The tally of [`search_parity`].
#[derive(Default)]
struct Parity {
    games: usize,
    unpaired: usize,
    same_records: usize,
    same_outcomes: usize,
    events: usize,
    same_events: usize,
    unmatched_events: usize,
    largest_world_gap: f64,
    largest_mean_gap: f64,
    first_parting: Option<Value>,
}

impl Parity {
    fn compare_events(&mut self, deal: u64, game_a: &Value, game_b: &Value) {
        let empty = Vec::new();
        let events_a = game_a["search_events"].as_array().unwrap_or(&empty);
        let events_b = game_b["search_events"].as_array().unwrap_or(&empty);
        self.unmatched_events += events_a.len().abs_diff(events_b.len());
        let both = events_a.len().min(events_b.len());
        for (n, (event_a, event_b)) in events_a.iter().zip(events_b).enumerate() {
            self.events += 1;
            let same = ["decision_seq", "candidates", "chosen", "action"]
                .iter()
                .all(|key| event_a[key] == event_b[key]);
            if !same {
                if self.first_parting.is_none() {
                    self.first_parting = Some(parting(deal, Some(event_a), "search"));
                }
                // Games part at their first different answer: the events
                // after it are other positions, never compared.
                self.unmatched_events += 2 * (both - n - 1);
                return;
            }
            self.same_events += 1;
            if let Some(gap) = largest_gap(&event_a["worlds"], &event_b["worlds"]) {
                self.largest_world_gap = self.largest_world_gap.max(gap);
            }
            if let Some(gap) = largest_gap(&event_a["means"], &event_b["means"]) {
                self.largest_mean_gap = self.largest_mean_gap.max(gap);
            }
        }
    }

    fn into_json(self, a: &Path, b: &Path) -> Value {
        json!({
            "contract": "search_parity_v1",
            "a": a,
            "b": b,
            "games": self.games,
            "unpaired_games": self.unpaired,
            "same_records": self.same_records,
            "same_outcomes": self.same_outcomes,
            "searched_decisions_compared": self.events,
            "same_searched_decisions": self.same_events,
            "unmatched_searched_decisions": self.unmatched_events,
            "largest_world_score_gap": self.largest_world_gap,
            "largest_mean_gap": self.largest_mean_gap,
            "identical": self.games > 0
                && self.unpaired == 0
                && self.same_records == self.games
                && self.same_events == self.events
                && self.unmatched_events == 0,
            "first_parting": self.first_parting,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(seq: u64, chosen: u64, worlds: &[[f64; 2]]) -> Value {
        json!({"decision_seq": seq, "candidates": ["PassPriority", {"PlayLand": {"card": 5}}],
            "chosen": chosen, "action": "PassPriority",
            "worlds": worlds, "means": [worlds[0][0], worlds[0][1]]})
    }

    fn write(dir: &Path, games: &[Value]) {
        std::fs::create_dir_all(dir).unwrap();
        let lines: Vec<String> = games.iter().map(Value::to_string).collect();
        std::fs::write(dir.join("games.jsonl"), lines.join("\n")).unwrap();
    }

    fn game(deal: u64, sha: &str, events: &[Value]) -> Value {
        json!({"i": deal, "record_sha256": sha, "outcome": "win", "search_events": events})
    }

    #[test]
    fn same_searches_with_noisy_scores_are_identical_up_to_their_gap() {
        let root = std::env::temp_dir().join(format!("search-parity-{}", std::process::id()));
        let (a, b) = (root.join("a"), root.join("b"));
        write(&a, &[game(0, "x", &[event(5, 0, &[[0.5, 0.25]])])]);
        write(
            &b,
            &[game(0, "x", &[event(5, 0, &[[0.5, 0.25 + 1.0 / 1024.0]])])],
        );
        let report = search_parity(&a, &b).unwrap();
        assert_eq!(report["identical"], json!(true));
        assert_eq!(report["same_searched_decisions"], json!(1));
        assert!((report["largest_world_score_gap"].as_f64().unwrap() - 1.0 / 1024.0).abs() < 1e-12);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn another_choice_parts_the_runs_there() {
        let root = std::env::temp_dir().join(format!("search-parity-c-{}", std::process::id()));
        let (a, b) = (root.join("a"), root.join("b"));
        write(
            &a,
            &[game(
                3,
                "x",
                &[event(5, 0, &[[0.5, 0.25]]), event(9, 0, &[[0.5, 0.5]])],
            )],
        );
        write(
            &b,
            &[game(
                3,
                "y",
                &[event(5, 1, &[[0.5, 0.25]]), event(9, 0, &[[0.5, 0.5]])],
            )],
        );
        let report = search_parity(&a, &b).unwrap();
        assert_eq!(report["identical"], json!(false));
        assert_eq!(report["first_parting"]["deal"], json!(3));
        assert_eq!(report["first_parting"]["why"], json!("record"));
        assert_eq!(report["same_searched_decisions"], json!(0));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
