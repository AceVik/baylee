//! The four engine decks get the printings they were designed with.
//!
//! # What was already here
//!
//! Migration 2 seeded Kenrith, Breya, Kess and Tayam as bare card names —
//! `"1 Birds of Paradise"` — which was the honest shape at the time: a deck
//! outlives the pool it was built against, and there was nothing to say about
//! *which* Birds of Paradise. They come from a report the owner commissioned,
//! *Commander-Decks für eine MTG-Engine* (14.09.2026), and that report names a
//! printing for every one of its 400 slots as a Scryfall link. Held against
//! the seeded lists card for card and in order, the two agree exactly: these
//! are the same four decks, told with less detail.
//!
//! So this migration adds no deck. It replaces those 396 rows with 400 that
//! carry the set and collector number the pod was built with, in the
//! `baylee_core::deckrow` spelling the other four house decks already use.
//! Four hundred rather than 396 because the leader joins them: a commander
//! named only in `commanders` has no row, and a row is where the printing
//! lives. `decks::from_lines` moves a named row out of the library instead of
//! copying it, so the deck is still a hundred cards.
//! The lists live in `data/decks/{kenrith,breya,kess,tayam}.txt`, with the
//! report's own form beside them in `data/decks/raw/astra-*.txt`;
//! `scripts/forge/resolve_printings.py` is what turned one into the other and
//! `cargo run -p xtask -- deck-check` is what says a row still parses and
//! still names a card this pool has.
//!
//! All 400 printings resolve against the catalogue with neither the set nor
//! the collector number lost — which is worth recording rather than assuming,
//! because a pod that reaches into Reality Fracture and Star Trek on purpose,
//! for `prepare`, `Station` and `Assimilate`, is exactly where a printing
//! would be expected to be missing.
//!
//! # The card that was not a spelling mistake
//!
//! Migration 3 repaired two rows that named both faces of a card, and said
//! Kenrith had been playing 98 cards instead of 99. That was one card short of
//! the truth: `Invasion of Ikoria` is spelled correctly and simply was not in
//! this repository's pool, so it resolved to nothing for the same reason and
//! with the same silence. It is in `data/card-pool.txt` now, which is what
//! actually makes that deck 99 cards — a migration could not have fixed it,
//! because nothing was wrong with the row.
//!
//! The lesson is the one worth keeping: a seeded name is checked against
//! nothing. `deck-check` is the reader that asks, and a deck that is seeded
//! and never read through it can be wrong in a way no test sees.
//!
//! # What this deliberately does not touch
//!
//! `version` stays at 1. A house deck belongs to nobody and nobody has edited
//! it; version 1 means "as this schema ships it", and the same deck told in
//! more detail has not become a second version of itself. A player who already
//! copied one of these decks keeps their copy exactly as it was — a copy is
//! theirs, and a migration is not a reason to reach into somebody's account.

use sea_orm_migration::prelude::*;

use super::decklist::{self, Decklist};
use super::house_decks::{BREYA, KENRITH, KESS, TAYAM};

/// The pod, as `data/decks/` now holds it, beside the rows migration 2 seeded.
///
/// The name is the file's own `[deck:…]` header and is what identifies the row
/// in the table, so a deck cannot be called one thing here and another there.
const DECKS: [(&str, &[&str]); 4] = [
    (include_str!("../../../../data/decks/kenrith.txt"), KENRITH),
    (include_str!("../../../../data/decks/breya.txt"), BREYA),
    (include_str!("../../../../data/decks/kess.txt"), KESS),
    (include_str!("../../../../data/decks/tayam.txt"), TAYAM),
];

/// The migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for (text, _) in DECKS {
            let list = Decklist::parse(text);
            decklist::replace_rows(manager, &list.name, &list.main, &list.commanders).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for (text, seeded) in DECKS {
            let list = Decklist::parse(text);
            // Back to the names-only rows, with the two front-face repairs
            // left in place: `down` undoes what this migration changed, not
            // what the one before it corrected, and putting an unresolvable
            // card name back is not a state anybody wants to return to. The
            // leader goes back outside the rows, named by the deck — which is
            // the same string, because migration 2 named each deck after it.
            let cards: Vec<String> = seeded.iter().map(|row| decklist::repaired(row)).collect();
            let leaders = vec![list.name.clone()];
            decklist::replace_rows(manager, &list.name, &cards, &leaders).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_deck_file_is_a_hundred_cards_under_one_name() {
        let files: Vec<(&str, &str, &str)> = DECKS
            .iter()
            .map(|(text, _)| (*text, "commander", "the pod"))
            .collect();
        decklist::assert_seedable(&files);
    }

    #[test]
    fn the_printed_rows_are_the_seeded_rows_with_a_printing_added() {
        // The whole claim of this migration in one assertion: same cards,
        // same order, more detail. The list carries its leader as row one and
        // the seed carried it outside the rows, so that row is what the two
        // differ by and everything behind it has to line up exactly. If the
        // report and the seed ever disagree about a card, this is where it
        // stops being a silent substitution.
        for (text, seeded) in DECKS {
            let list = Decklist::parse(text);
            assert_eq!(
                list.main.len(),
                seeded.len() + 1,
                "{}: {} rows against {} seeded plus the leader",
                list.name,
                list.main.len(),
                seeded.len()
            );
            assert!(
                list.names_a_row(&list.name),
                "{}: row one is not the leader",
                list.name
            );
            for (row, was) in list.main[1..].iter().zip(seeded) {
                let bare = decklist::repaired(was);
                assert!(
                    row == &bare || row.starts_with(&format!("{bare} (")),
                    "{}: {row} is not {bare} with a printing",
                    list.name
                );
            }
        }
    }

    #[test]
    fn the_deck_this_migration_names_is_the_deck_that_was_seeded() {
        // `replace_rows` keys on the name, so a renamed file would update
        // nothing at all and report success — the one way this migration can
        // fail in silence.
        let names: Vec<String> = DECKS
            .iter()
            .map(|(text, _)| Decklist::parse(text).name)
            .collect();
        for expected in [
            "Kenrith, the Returned King",
            "Breya, Etherium Shaper",
            "Kess, Dissident Mage",
            "Tayam, Luminous Enigma",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "no deck file is called {expected}; the ones here are {names:?}"
            );
        }
    }
}
