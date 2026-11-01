//! Misstep — {1}{U} — Sorcery
//! Oracle: Creatures target player controls don't untap during that player's next untap step.
//! Set: MMQ #88 — Mercadian Masques | Scryfall ID: 8e23f5a1-bf3e-41e0-875f-fc2f8508e69f | Oracle ID: 4ee994a1-fd3b-43cf-be3c-9c705e60754a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MISSTEP,
    oracle_id = "4ee994a1-fd3b-43cf-be3c-9c705e60754a",
    scryfall_id = "8e23f5a1-bf3e-41e0-875f-fc2f8508e69f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Misstep",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
