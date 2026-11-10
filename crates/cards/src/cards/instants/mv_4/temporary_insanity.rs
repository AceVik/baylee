//! Temporary Insanity — {3}{R} — Instant
//! Oracle: Untap target creature with power less than the number of cards in your graveyard and gain control of it until end of turn. That creature gains haste until end of turn.
//! Set: DDG #73 — Duel Decks: Knights vs. Dragons | Scryfall ID: d37747ce-d532-469b-9963-ad25c95a8361 | Oracle ID: 4f9c66f9-f5d4-499d-9325-e779e6612bca
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEMPORARY_INSANITY,
    oracle_id = "4f9c66f9-f5d4-499d-9325-e779e6612bca",
    scryfall_id = "d37747ce-d532-469b-9963-ad25c95a8361",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Temporary Insanity",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
