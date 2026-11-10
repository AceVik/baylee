//! A paired comparison broken down by deck: for each deck the net piloted, and
//! for each deck it met, the deals, the arm's and the reference's wins and the
//! difference per deal. Read off the rows [`super::arms::compare_arm`] pairs,
//! each with its `decks`, `focal_seat`, `net_utility` (the arm) and
//! `base_utility` (the reference).
// Counts of deals, well below f64's exact 2^53 range.
#![allow(clippy::cast_precision_loss)]
use super::{flag, number};
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Which deals a breakdown reads: those a deck pilots, those a deck opposes.
#[derive(Clone, Debug, Default)]
pub struct DeckFilter {
    /// Only deals the net played with this deck.
    pub pilot: Option<String>,
    /// Only deals with this deck among the net's opponents.
    pub opponent: Option<String>,
}

#[derive(Default)]
struct Tally {
    deals: usize,
    arm: f64,
    reference: f64,
    better: usize,
    worse: usize,
    capped: usize,
}

impl Tally {
    fn add(&mut self, arm: f64, reference: f64, capped: bool) {
        self.deals += 1;
        self.arm += arm;
        self.reference += reference;
        self.better += usize::from(arm > reference);
        self.worse += usize::from(arm < reference);
        self.capped += usize::from(capped);
    }

    fn row(&self, deck: &str, small: usize) -> Value {
        let deals = self.deals.max(1) as f64;
        json!({"deck": deck, "deals": self.deals, "arm_wins": self.arm,
            "reference_wins": self.reference,
            "difference": (self.arm - self.reference) / deals,
            "better": self.better, "worse": self.worse, "capped": self.capped,
            "small": self.deals < small})
    }
}

fn decks(row: &Value) -> Result<Vec<&str>> {
    row["decks"]
        .as_array()
        .context("a row without its decks")?
        .iter()
        .map(|deck| deck.as_str().context("a deck that is not a key"))
        .collect()
}

/// The net's deck and its opponents' at one deal. With `teams` (one per
/// seat, as the summary's `teams`), a teammate is no opponent.
fn seats<'a>(row: &'a Value, teams: &[u64]) -> Result<(&'a str, Vec<&'a str>)> {
    let decks = decks(row)?;
    let focal = usize::try_from(
        row["focal_seat"]
            .as_u64()
            .context("a row without its focal seat")?,
    )?;
    let pilot = *decks.get(focal).context("a focal seat past the decks")?;
    let team = teams.get(focal).copied().filter(|team| *team != 0);
    let opponents = decks
        .iter()
        .enumerate()
        .filter(|(seat, _)| *seat != focal && (team.is_none() || teams.get(*seat).copied() != team))
        .map(|(_, deck)| *deck)
        .collect();
    Ok((pilot, opponents))
}

fn sorted(tallies: &BTreeMap<String, Tally>, small: usize) -> Vec<Value> {
    let mut rows: Vec<(f64, usize, Value)> = tallies
        .iter()
        .map(|(deck, tally)| {
            let row = tally.row(deck, small);
            let size = row["difference"].as_f64().unwrap_or(0.0).abs();
            (size, tally.deals, row)
        })
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.cmp(&a.1)));
    rows.into_iter().map(|(_, _, row)| row).collect()
}

/// The breakdown of `rows` (paired rows with their focal seats) under
/// `filter`: by the deck the net piloted and by each deck it met, the largest
/// difference first. A deck with fewer than `small` deals is marked `small`.
///
/// # Errors
/// A row without its decks, focal seat, utilities or cap flags.
pub fn by_deck(rows: &[Value], teams: &[u64], filter: &DeckFilter, small: usize) -> Result<Value> {
    let mut pilots: BTreeMap<String, Tally> = BTreeMap::new();
    let mut opponents: BTreeMap<String, Tally> = BTreeMap::new();
    let mut deals = 0;
    for row in rows {
        let (pilot, met) = seats(row, teams)?;
        if filter
            .pilot
            .as_deref()
            .is_some_and(|wanted| wanted != pilot)
            || filter
                .opponent
                .as_deref()
                .is_some_and(|wanted| !met.contains(&wanted))
        {
            continue;
        }
        deals += 1;
        let arm = number(row, "net_utility")?;
        let reference = number(row, "base_utility")?;
        let capped = flag(row, "net_capped")? || flag(row, "base_capped")?;
        pilots
            .entry(pilot.to_owned())
            .or_default()
            .add(arm, reference, capped);
        let mut seen: Vec<&str> = met.clone();
        seen.sort_unstable();
        seen.dedup();
        for opponent in seen {
            opponents
                .entry(opponent.to_owned())
                .or_default()
                .add(arm, reference, capped);
        }
    }
    Ok(json!({
        "filter": {"pilot": filter.pilot, "opponent": filter.opponent},
        "deals": deals,
        "small_below": small,
        "by_pilot": sorted(&pilots, small),
        "by_opponent": sorted(&opponents, small),
    }))
}

/// `by_deck`'s rows as a Markdown table, `*` marking a small deck.
#[must_use]
pub fn markdown(title: &str, rows: &[Value]) -> String {
    let mut out = format!(
        "| {title} | deals | arm wins | ref wins | diff/deal | better/worse |\n| --- | --- | --- | --- | --- | --- |\n"
    );
    for row in rows {
        // Writing to a `String` cannot fail.
        let _ = writeln!(
            out,
            "| {}{} | {} | {:.1} | {:.1} | {:+.1} pp | {}/{} |",
            row["deck"].as_str().unwrap_or(""),
            if row["small"] == true { " *" } else { "" },
            row["deals"],
            row["arm_wins"].as_f64().unwrap_or(0.0),
            row["reference_wins"].as_f64().unwrap_or(0.0),
            row["difference"].as_f64().unwrap_or(0.0) * 100.0,
            row["better"],
            row["worse"],
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(decks: [&str; 2], focal: u64, arm: f64, reference: f64) -> Value {
        json!({"decks": decks, "focal_seat": focal, "net_utility": arm,
            "base_utility": reference, "net_capped": false, "base_capped": false})
    }

    fn rows() -> Vec<Value> {
        vec![
            row(["victory", "elves"], 0, 1.0, 0.0),
            row(["victory", "elves"], 1, 0.0, 0.0),
            row(["burn", "victory"], 1, 1.0, 0.0),
            row(["burn", "victory"], 0, 0.0, 1.0),
            row(["burn", "elves"], 0, 1.0, 1.0),
        ]
    }

    #[test]
    fn decks_are_tallied_by_pilot_and_by_opponent_largest_difference_first() {
        let report = by_deck(&rows(), &[], &DeckFilter::default(), 3).unwrap();
        assert_eq!(report["deals"], 5);
        let pilots = report["by_pilot"].as_array().unwrap();
        assert_eq!(pilots[0]["deck"], "victory");
        assert_eq!(pilots[0]["deals"], 2);
        assert_eq!(pilots[0]["arm_wins"], 2.0);
        assert_eq!(pilots[0]["difference"], 1.0);
        assert_eq!(pilots[0]["small"], true);
        let burn = pilots.iter().find(|row| row["deck"] == "burn").unwrap();
        assert_eq!(burn["difference"], -0.5);
        let opponents = report["by_opponent"].as_array().unwrap();
        let victory = opponents
            .iter()
            .find(|row| row["deck"] == "victory")
            .unwrap();
        assert_eq!(victory["deals"], 2);
        assert_eq!(victory["better"], 0);
        assert_eq!(victory["worse"], 1);
    }

    #[test]
    fn a_named_opponent_keeps_only_its_deals() {
        let filter = DeckFilter {
            opponent: Some("victory".into()),
            ..DeckFilter::default()
        };
        let report = by_deck(&rows(), &[], &filter, 1).unwrap();
        assert_eq!(report["deals"], 2);
        let pilots = report["by_pilot"].as_array().unwrap();
        assert_eq!(pilots.len(), 2);
        assert!(pilots.iter().all(|row| row["deck"] != "victory"));
    }

    #[test]
    fn a_named_pilot_keeps_only_its_deals() {
        let filter = DeckFilter {
            pilot: Some("burn".into()),
            ..DeckFilter::default()
        };
        let report = by_deck(&rows(), &[], &filter, 1).unwrap();
        assert_eq!(report["deals"], 2);
        let opponents = report["by_opponent"].as_array().unwrap();
        assert_eq!(opponents.len(), 2);
    }

    /// At a team table a teammate's deck is not an opponent's.
    #[test]
    fn a_teammate_is_no_opponent() {
        let row = json!({"decks": ["a", "b", "c", "d"], "focal_seat": 0, "net_utility": 1.0,
            "base_utility": 0.0, "net_capped": false, "base_capped": false});
        let report = by_deck(&[row], &[1, 2, 1, 2], &DeckFilter::default(), 1).unwrap();
        let met: Vec<&str> = report["by_opponent"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["deck"].as_str().unwrap())
            .collect();
        assert_eq!(met.len(), 2);
        assert!(met.contains(&"b") && met.contains(&"d"));
    }

    #[test]
    fn the_markdown_marks_a_small_deck() {
        let report = by_deck(&rows(), &[], &DeckFilter::default(), 3).unwrap();
        let text = markdown("pilot", report["by_pilot"].as_array().unwrap());
        assert!(
            text.contains("| victory * | 2 | 2.0 | 0.0 | +100.0 pp | 2/0 |"),
            "{text}"
        );
    }
}
