//! Decompose — {1}{B} — Sorcery
//! Oracle: Exile up to three target cards from a single graveyard.
//! Set: DDK #20 — Duel Decks: Sorin vs. Tibalt | Scryfall ID: f4868145-fb8b-4624-8554-ddbab3d5a43c | Oracle ID: 12c2b6bb-05f8-4291-b9a5-c4ddb7a69b9e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DECOMPOSE,
    oracle_id = "12c2b6bb-05f8-4291-b9a5-c4ddb7a69b9e",
    scryfall_id = "f4868145-fb8b-4624-8554-ddbab3d5a43c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Decompose",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
