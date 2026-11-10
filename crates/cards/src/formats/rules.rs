//! The one validation function: a deck, a format and a list in, what is wrong
//! with it out, each problem with its card and reason.
//!
//! Pure, deterministic and quick enough for every edit: no I/O, and nothing
//! it reads changes during a call. The deck builder calls it live and the
//! gateway calls it where a list guards a table, and the gateway's answer is
//! the one that counts (`docs/banlists-and-formats.md`). The engine stays out:
//! it plays whatever it is handed.
//!
//! # A list replaces the format's bans, not its pool
//!
//! Without a list a format's own verdicts stand: banned out, restricted one
//! copy, `not_legal` out. A list says what is banned and restricted instead,
//! because that is what starting from a copy of an official list and taking a
//! card off it means; the format's pool still stands (`not_legal`), so a
//! Modern list with one card unbanned is not a Modern list with every card
//! ever printed. A list for `freeform` keeps to no format at all.

use std::collections::BTreeMap;

use baylee_cards_dsl::Coverage;
use baylee_cardtext::{Legality, ListRules, ListStatus, PoolFacts, Problem, Report};
use baylee_core::ids::CardIndex;

use super::{FREEFORM, shape_of, shape_problems};
use crate::decks::LoadedDeck;

/// A deck as a check reads it: one entry per copy.
#[derive(Clone, Copy, Debug)]
pub struct DeckCards<'a> {
    /// The library, leaders not among them.
    pub main: &'a [CardIndex],
    /// The sideboard.
    pub side: &'a [CardIndex],
    /// The commanders.
    pub leaders: &'a [CardIndex],
}

/// What a deck is checked against.
#[derive(Clone, Copy, Debug)]
pub struct Rules<'a> {
    /// The format: Scryfall's key, or [`FREEFORM`] for none.
    pub format: &'a str,
    /// What each format says about each card; `None` where there is no table
    /// to read, and the report then says so.
    pub facts: Option<&'a PoolFacts>,
    /// The chosen list, whose word stands in for the format's bans.
    pub list: Option<&'a ListRules>,
}

/// Everything wrong with `deck` under `rules`, and what could not be asked.
#[must_use]
pub fn validate(deck: DeckCards<'_>, rules: Rules<'_>) -> Report {
    let mut report = Report::default();
    let free = rules.format.eq_ignore_ascii_case(FREEFORM);
    if !free {
        match shape_of(rules.format) {
            Some(shape) => {
                report.problems = shape_problems(shape, deck.main, deck.side, deck.leaders);
            }
            None => report.unchecked.shape = true,
        }
    }
    let copies = held(deck);
    // The format's table is needed for its pool whatever the list says, and
    // not at all where there is no format to ask about.
    let table = rules
        .facts
        .filter(|table| !free && !table.is_empty() && table.knows_format(rules.format));
    report.unchecked.statuses = !free && table.is_none();
    for (&card, &have) in &copies {
        if let Some(list) = rules.list {
            list_problems(card, have, deck.leaders, list, &mut report);
        }
        if let Some(table) = table {
            status_problems(card, have, rules, table, &mut report);
        }
        if crate::by_index(card).is_some_and(|def| def.coverage == Coverage::Unimplemented) {
            report.unscripted.push(card.get());
        }
    }
    report
}

/// [`validate`] over a deck as the gateway loads it.
#[must_use]
pub fn validate_loaded(deck: &LoadedDeck, rules: Rules<'_>) -> Report {
    let cards = |list: &[crate::decks::DeckCard]| -> Vec<CardIndex> {
        list.iter().map(|card| card.index).collect()
    };
    validate(
        DeckCards {
            main: &cards(&deck.main),
            side: &cards(&deck.sideboard),
            leaders: &cards(&deck.commanders),
        },
        rules,
    )
}

/// How many copies of each card the deck holds in all, in index order.
fn held(deck: DeckCards<'_>) -> BTreeMap<CardIndex, u32> {
    let mut copies: BTreeMap<CardIndex, u32> = BTreeMap::new();
    for card in deck.main.iter().chain(deck.side).chain(deck.leaders) {
        *copies.entry(*card).or_default() += 1;
    }
    copies
}

/// What the list says about `card`.
fn list_problems(
    card: CardIndex,
    have: u32,
    leaders: &[CardIndex],
    list: &ListRules,
    report: &mut Report,
) {
    let Some(def) = crate::by_index(card) else {
        return;
    };
    let card_number = card.get();
    match list.status(def.oracle_id) {
        Some(ListStatus::Banned) => report
            .problems
            .push(Problem::ListBanned { card: card_number }),
        Some(ListStatus::Restricted) if have > 1 => report.problems.push(Problem::ListRestricted {
            card: card_number,
            have,
        }),
        Some(ListStatus::BannedAsCommander) if leaders.contains(&card) => report
            .problems
            .push(Problem::ListBannedAsCommander { card: card_number }),
        Some(ListStatus::Restricted | ListStatus::BannedAsCommander) | None => {}
    }
}

/// What the format's table says about `card`. With a list only its pool
/// stands: the list has already said what is banned and restricted.
fn status_problems(
    card: CardIndex,
    have: u32,
    rules: Rules<'_>,
    table: &PoolFacts,
    report: &mut Report,
) {
    let card_number = card.get();
    let verdict = table
        .of(card_number)
        .and_then(|facts| facts.legality(rules.format));
    match verdict {
        Some(Legality::NotLegal) => report
            .problems
            .push(Problem::NotLegal { card: card_number }),
        Some(Legality::Banned) if rules.list.is_none() => {
            report.problems.push(Problem::Banned { card: card_number });
        }
        Some(Legality::Restricted) if rules.list.is_none() && have > 1 => {
            report.problems.push(Problem::Restricted {
                card: card_number,
                have,
            });
        }
        Some(Legality::Legal | Legality::Banned | Legality::Restricted) => {}
        None => report.unchecked.cards.push(card_number),
    }
}
