//! Breakthrough — {X}{U} — Sorcery
//! Oracle: Draw four cards, then choose X cards in your hand and discard the rest.
//! Set: 2X2 #41 — Double Masters 2022 | Scryfall ID: 22075f41-3aeb-406d-99b6-88ddae992d0b | Oracle ID: c6b26c1f-121f-4b7c-921e-429b623ca934
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BREAKTHROUGH,
    oracle_id = "c6b26c1f-121f-4b7c-921e-429b623ca934",
    scryfall_id = "22075f41-3aeb-406d-99b6-88ddae992d0b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Breakthrough",
        mana_cost = mana!("{X}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
