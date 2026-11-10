//! `eval4 --write-deals`: the deals whose net pilots chosen decks, out of the
//! decks the run's own flags select and seated by its own schedule
//! (`baylee_train::deals4`), so `--only-deals` plays exactly those.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context as _, ensure};
use baylee_train::deals4::{DeckSchedule, deals_piloting};
use baylee_train::housedeck::HouseDeck;

/// The deck keys `--pilot-decks` names: a comma list, or `@FILE` holding a
/// JSON list of keys.
pub(super) fn pilot_keys(spec: &str) -> anyhow::Result<BTreeSet<String>> {
    let keys: Vec<String> = match spec.strip_prefix('@') {
        Some(path) => serde_json::from_slice(
            &std::fs::read(path).with_context(|| format!("reading the deck list {path}"))?,
        )
        .with_context(|| format!("{path} is not a JSON list of deck keys"))?,
        None => spec
            .split(',')
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .map(str::to_owned)
            .collect(),
    };
    ensure!(!keys.is_empty(), "--pilot-decks names no deck");
    Ok(keys.into_iter().collect())
}

/// What a selection holds, by pilot deck: its deals, its opponents and its seats.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Selection {
    pub deals: Vec<u64>,
    /// Pilot deck → opponent deck → deals.
    pub opponents: BTreeMap<String, BTreeMap<String, usize>>,
    /// Pilot deck → net seat → deals.
    pub seats: BTreeMap<String, BTreeMap<usize, usize>>,
}

/// The deals of `first..first + count` whose net pilots one of `pilots`.
///
/// # Errors
/// A pilot no deck of the run is keyed by, or a schedule the table refuses.
pub(super) fn select(
    decks: &[HouseDeck],
    schedule: DeckSchedule,
    seats: usize,
    first: u64,
    count: u64,
    pilots: &BTreeSet<String>,
) -> anyhow::Result<Selection> {
    let missing: Vec<&String> = pilots
        .iter()
        .filter(|key| !decks.iter().any(|deck| &deck.key == *key))
        .collect();
    ensure!(
        missing.is_empty(),
        "these flags select {} decks and none is keyed {missing:?}",
        decks.len()
    );
    let chosen: BTreeSet<usize> = (0..decks.len())
        .filter(|index| pilots.contains(&decks[*index].key))
        .collect();
    let deals = deals_piloting(schedule, seats, decks.len(), first, count, |deck| {
        chosen.contains(&deck)
    })?;
    let mut selection = Selection::default();
    for deal in &deals {
        let pilot = &decks[deal.focal_deck()].key;
        selection.deals.push(deal.deal);
        for (seat, &deck) in deal.decks.iter().enumerate() {
            if seat != deal.focal_seat {
                *selection
                    .opponents
                    .entry(pilot.clone())
                    .or_default()
                    .entry(decks[deck].key.clone())
                    .or_default() += 1;
            }
        }
        *selection
            .seats
            .entry(pilot.clone())
            .or_default()
            .entry(deal.focal_seat)
            .or_default() += 1;
    }
    Ok(selection)
}

/// Writes `selection`'s deals to `out` and says what they hold.
pub(super) fn write(out: &Path, selection: &Selection, decks: usize) -> anyhow::Result<()> {
    ensure!(
        !selection.deals.is_empty(),
        "no deal of the range pilots these decks: play more deals"
    );
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out, serde_json::to_vec(&selection.deals)?)?;
    println!(
        "{} deals of {decks} decks to {}",
        selection.deals.len(),
        out.display()
    );
    for (pilot, opponents) in &selection.opponents {
        let deals: usize = opponents.values().sum();
        let least = opponents.values().min().copied().unwrap_or(0);
        let most = opponents.values().max().copied().unwrap_or(0);
        println!(
            "{pilot}: {deals} deals, {} opponents ({least}..{most} deals each), seats {:?}",
            opponents.len(),
            selection.seats.get(pilot).cloned().unwrap_or_default()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decks(count: usize) -> Vec<HouseDeck> {
        let text = std::fs::read_to_string(
            baylee_train::working::repo_root().join("data/decks/eval/azorius-control.txt"),
        )
        .unwrap();
        let deck = HouseDeck::parse_any("x", &text).unwrap();
        (0..count)
            .map(|index| HouseDeck {
                key: format!("deck/{index}"),
                deck: deck.deck.clone(),
            })
            .collect()
    }

    #[test]
    fn a_round_robin_selection_meets_every_other_deck_from_both_seats() {
        let decks = decks(6);
        let pilots = BTreeSet::from(["deck/2".to_owned()]);
        let selection = select(&decks, DeckSchedule::RoundRobin, 2, 0, 2 * 6 * 5, &pilots).unwrap();
        assert_eq!(selection.deals.len(), 10);
        let opponents = &selection.opponents["deck/2"];
        assert_eq!(opponents.len(), 5);
        assert!(opponents.values().all(|count| *count == 2));
        assert_eq!(selection.seats["deck/2"], BTreeMap::from([(0, 5), (1, 5)]));
    }

    /// The same deals as the evaluator's own fixed seating: deck `d mod n`
    /// pilots deal `d` with two seats.
    #[test]
    fn a_fixed_selection_is_every_nth_deal() {
        let decks = decks(6);
        let pilots = BTreeSet::from(["deck/4".to_owned()]);
        let selection = select(&decks, DeckSchedule::Fixed, 2, 0, 30, &pilots).unwrap();
        assert_eq!(selection.deals, [4, 10, 16, 22, 28]);
        assert_eq!(selection.opponents["deck/4"].len(), 1);
    }

    #[test]
    fn a_pilot_the_flags_do_not_select_is_refused() {
        let decks = decks(4);
        let pilots = BTreeSet::from(["house/victory".to_owned()]);
        let error = select(&decks, DeckSchedule::Fixed, 2, 0, 8, &pilots).unwrap_err();
        assert!(error.to_string().contains("house/victory"), "{error}");
    }

    #[test]
    fn pilot_keys_read_a_comma_list_or_a_file() {
        assert_eq!(
            pilot_keys("a/b, c/d").unwrap(),
            BTreeSet::from(["a/b".to_owned(), "c/d".to_owned()])
        );
        let path = std::env::temp_dir().join(format!("pilots-{}.json", std::process::id()));
        std::fs::write(&path, br#"["x/y"]"#).unwrap();
        assert_eq!(
            pilot_keys(&format!("@{}", path.display())).unwrap(),
            BTreeSet::from(["x/y".to_owned()])
        );
        std::fs::remove_file(path).unwrap();
        assert!(pilot_keys(" , ").is_err());
    }
}
