//! Matched historical arena outcomes; incomplete games remain excluded explicitly.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Deal(u64, u64),
    Game(u64),
}
struct Outcome {
    won: bool,
    deck: Value,
    against: Value,
}
fn outcomes(root: &Path) -> Result<(BTreeMap<Key, Outcome>, u32)> {
    let mut rows = BTreeMap::new();
    let mut keys = std::collections::BTreeSet::new();
    let mut incomplete = 0_u32;
    for line in BufReader::new(File::open(root.join("games.jsonl"))?).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let row: Value = serde_json::from_str(&line)?;
        if row.get("role").and_then(Value::as_str).unwrap_or("net") != "net" {
            continue;
        }
        let key = if let Some(deal) = row.get("deal") {
            Key::Deal(
                deal.as_u64().context("arena deal")?,
                row["net_seat"].as_u64().context("arena seat")?,
            )
        } else {
            Key::Game(row["i"].as_u64().context("arena game")?)
        };
        let stamp = match &key {
            Key::Deal(d, s) => format!("deal:{d}:{s}"),
            Key::Game(g) => format!("game:{g}"),
        };
        ensure!(keys.insert(stamp), "duplicate arena outcome identity");
        match row["outcome"].as_str() {
            Some("win" | "loss") => {
                rows.insert(
                    key,
                    Outcome {
                        won: row["outcome"] == "win",
                        deck: row["deck"].clone(),
                        against: row["against"].clone(),
                    },
                );
            }
            Some(_) => {
                incomplete = incomplete
                    .checked_add(1)
                    .context("arena incomplete count overflow")?;
            }
            None => anyhow::bail!("missing arena outcome"),
        }
    }
    Ok((rows, incomplete))
}
/// Two-sided exact-binomial sign-test formula, evaluated stably in floating point.
#[must_use]
pub fn sign_test(a: u32, b: u32) -> f64 {
    let n = u64::from(a) + u64::from(b);
    if n == 0 {
        return 1.;
    }
    let k = a.min(b);
    let nf = f64::from(a) + f64::from(b);
    let mut log_probability = -nf * std::f64::consts::LN_2;
    for j in 1..=k {
        log_probability += (nf - f64::from(j) + 1.).ln() - f64::from(j).ln();
    }
    let mut relative = 1.;
    let mut tail = 1.;
    for j in (1..=k).rev() {
        relative *= f64::from(j) / (nf - f64::from(j) + 1.);
        tail += relative;
    }
    (2. * log_probability.exp() * tail).min(1.)
}
/// Compare only matching completed historical deals/seats or legacy game IDs.
/// # Errors
/// Rejects duplicate keys, malformed records and conflicting recorded decks/opponents.
pub fn compare(base: &Path, other: &Path) -> Result<Value> {
    let (left, left_incomplete) = outcomes(base)?;
    let (right, right_incomplete) = outcomes(other)?;
    let mut games = 0_u32;
    let mut base_wins = 0;
    let mut other_wins = 0;
    let mut only_base = 0;
    let mut only_other = 0;
    for (key, a) in &left {
        let Some(b) = right.get(key) else {
            continue;
        };
        ensure!(
            a.deck == b.deck && a.against == b.against,
            "paired arena deck/opponent differs"
        );
        games = games.checked_add(1).context("paired game count overflow")?;
        base_wins += u32::from(a.won);
        other_wins += u32::from(b.won);
        only_base += u32::from(a.won && !b.won);
        only_other += u32::from(b.won && !a.won);
    }
    let rate = |wins: u32| {
        if games == 0 {
            None
        } else {
            Some(f64::from(wins) / f64::from(games))
        }
    };
    Ok(
        json!({"base":base,"other":other,"games_both_finished":games,"base_wins":base_wins,"other_wins":other_wins,"base_rate":rate(base_wins),"other_rate":rate(other_wins),"won_only_base":only_base,"won_only_other":only_other,"sign_test_p":sign_test(only_base,only_other),"base_excluded_non_win_loss":left_incomplete,"other_excluded_non_win_loss":right_incomplete,"pairing_contract":"historical deal/seat or legacy game ID; matching engine and seed contract remains a prerequisite"}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_sign_formula_is_symmetric_and_stable() {
        assert_eq!(sign_test(0, 0), 1.);
        assert_eq!(sign_test(1, 1), 1.);
        assert!((sign_test(0, 5) - 0.0625).abs() < 1e-12);
        assert_eq!(sign_test(1, 7), sign_test(7, 1));
        assert!((sign_test(1000, 1000) - 1.).abs() < 1e-10);
        assert!(sign_test(0, 10000) < 1e-200);
    }
    #[test]
    fn pairing_separates_legacy_keys_caps_and_roles() -> Result<()> {
        let root = tempfile::tempdir()?;
        let a = root.path().join("a");
        let b = root.path().join("b");
        std::fs::create_dir(&a)?;
        std::fs::create_dir(&b)?;
        std::fs::write(
            a.join("games.jsonl"),
            "{\"deal\":1,\"net_seat\":0,\"outcome\":\"win\"}\n{\"deal\":2,\"net_seat\":0,\"outcome\":\"cap\"}\n{\"deal\":1,\"net_seat\":0,\"role\":\"house\",\"outcome\":\"loss\"}\n",
        )?;
        std::fs::write(
            b.join("games.jsonl"),
            "{\"deal\":1,\"net_seat\":0,\"outcome\":\"loss\"}\n{\"deal\":2,\"net_seat\":0,\"outcome\":\"win\"}\n",
        )?;
        let result = compare(&a, &b)?;
        assert_eq!(result["games_both_finished"], 1);
        assert_eq!(result["won_only_base"], 1);
        assert_eq!(result["base_excluded_non_win_loss"], 1);
        std::fs::write(b.join("games.jsonl"), "{\"i\":1,\"outcome\":\"win\"}\n")?;
        assert_eq!(compare(&a, &b)?["games_both_finished"], 0);
        std::fs::write(
            b.join("games.jsonl"),
            "{\"i\":1,\"outcome\":\"win\"}\n{\"i\":1,\"outcome\":\"loss\"}\n",
        )?;
        assert!(compare(&a, &b).is_err());
        Ok(())
    }
}
