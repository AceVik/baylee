//! Training-side re-export and [`Working`]-pool helpers for [`baylee_cards::deckgen`].

use std::collections::BTreeSet;

pub use baylee_cards::deckgen::*;
use baylee_core::ids::CardIndex;

use crate::working::Working;

/// The cards held out of every training deck: one in `every`,
/// basics never. What a net does with them is its play on unseen cards.
#[must_use]
pub fn held_out(working: &Working, every: u64) -> BTreeSet<CardIndex> {
    held_out_cards(working.cards.iter().copied(), every)
}

/// Builds a [`Pool`] of the working cards in `working`, less `excluded`.
#[must_use]
pub fn pool_for_working(working: &Working, excluded: &BTreeSet<CardIndex>) -> Pool {
    Pool::from_cards(working.cards.iter().copied(), excluded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_cards::formats::{self, Shape};
    use baylee_core::color::{Color, ColorSet};
    use baylee_core::generated::index::CRYPTIC_SPIRES;

    fn pool() -> &'static (Working, Pool) {
        static CACHE: std::sync::OnceLock<(Working, Pool)> = std::sync::OnceLock::new();
        CACHE.get_or_init(|| {
            let working = Working::scan(&crate::working::repo_root()).unwrap();
            let held = held_out(&working, 20);
            let pool = pool_for_working(&working, &held);
            (working, pool)
        })
    }

    /// Every shape and archetype builds decks of its shape, from working
    /// cards that are not held out, and the same seed builds the same deck.
    #[test]
    fn generated_decks_are_decks_of_working_cards() {
        let (working, pool) = pool();
        let held = held_out(working, 20);
        let mut built = 0;
        for shape in [Shape::Constructed, Shape::Highlander, Shape::Commander] {
            for archetype in Archetype::ALL {
                for seed in 0..6 {
                    let Ok(deck) = generate(pool, shape, archetype, seed) else {
                        continue;
                    };
                    built += 1;
                    assert!(formats::check(shape, &deck.main, &deck.leaders).is_empty());
                    for card in deck.distinct() {
                        assert!(working.check(card).is_ok(), "{card} does not work");
                        assert!(!held.contains(&card), "{card} is held out");
                    }
                    let parsed = crate::housedeck::HouseDeck::parse("generated", &deck.text())
                        .expect("every generated deck retains valid construction choices");
                    let mut parsed_main = parsed.deck.main;
                    parsed_main.sort_by_key(|card| card.index);
                    assert_eq!(parsed_main, deck.loaded().main);
                    let again = generate(pool, shape, archetype, seed).unwrap();
                    assert_eq!(deck.main, again.main);
                }
            }
        }
        assert!(built >= 60, "only {built} of 72 decks built");
    }

    #[test]
    fn some_constructed_decks_hold_more_than_sixty() {
        let (_, pool) = pool();
        let sizes: BTreeSet<usize> = (0..40)
            .filter_map(|s| generate(pool, Shape::Constructed, Archetype::Midrange, s).ok())
            .map(|d| d.main.len())
            .collect();
        assert!(sizes.contains(&60));
        assert!(sizes.iter().any(|n| *n > 60), "{sizes:?}");
    }

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn the_held_out_slice_is_a_twentieth_and_stable() {
        let (working, _) = pool();
        let held = held_out(working, 20);
        let share = held.len() as f64 / working.cards.len() as f64;
        assert!((0.03..0.08).contains(&share), "{share}");
        assert_eq!(held, held_out(working, 20));
    }

    #[test]
    fn spires_choices_survive_loaded_and_text_decks() {
        for colors in [ColorSet::of(Color::Red), ColorSet::ALL] {
            let deck = Generated {
                name: "Spires".into(),
                shape: Shape::Constructed,
                archetype: Archetype::Midrange,
                colors,
                main: vec![CRYPTIC_SPIRES; 4],
                leaders: Vec::new(),
            };
            let loaded = deck.loaded();
            let parsed = crate::housedeck::HouseDeck::parse("spires", &deck.text()).unwrap();
            assert_eq!(parsed.deck.main, loaded.main);
            for card in loaded.main {
                baylee_cards::print_rules::validate_choices(card.index, card.choices).unwrap();
                assert_eq!(card.choices.colors().len(), 2);
                assert!(!card.choices.colors().intersection(colors).is_empty());
            }
        }
    }

    #[test]
    fn the_text_is_a_house_deck_file() {
        let (_, pool) = pool();
        let deck = generate(pool, Shape::Commander, Archetype::Midrange, 3).unwrap();
        let parsed = crate::housedeck::HouseDeck::parse("gen", &deck.text()).unwrap();
        let mut parsed_main = parsed.deck.main;
        parsed_main.sort_by_key(|card| card.index);
        assert_eq!(parsed_main, deck.loaded().main);
        assert_eq!(parsed.deck.commanders.len(), 1);
    }
}
