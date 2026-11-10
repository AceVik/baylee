//! Noxious Vapors — {1}{B}{B} — Sorcery
//! Oracle: Each player reveals their hand, chooses one card of each color from it, then discards all other nonland cards.
//! Set: PLS #49 — Planeshift | Scryfall ID: e3cf9326-6e1c-4a05-abea-16d6b6cb2a6d | Oracle ID: cd16634e-b894-4cff-9d5b-408932863c1f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NOXIOUS_VAPORS,
    oracle_id = "cd16634e-b894-4cff-9d5b-408932863c1f",
    scryfall_id = "e3cf9326-6e1c-4a05-abea-16d6b6cb2a6d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Noxious Vapors",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
