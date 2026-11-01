//! Kavu Runner — {3}{R} — Creature — Kavu
//! Oracle: This creature has haste as long as no opponent controls a white or blue creature.
//! Set: INV #150 — Invasion | Scryfall ID: 2bc1b462-4e3c-47cc-87c5-f6e29dd70c01 | Oracle ID: 2c90b7fa-d5bf-4c46-92d5-3014ea1bec50
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KAVU_RUNNER,
    oracle_id = "2c90b7fa-d5bf-4c46-92d5-3014ea1bec50",
    scryfall_id = "2bc1b462-4e3c-47cc-87c5-f6e29dd70c01",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Kavu Runner",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KAVU],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
