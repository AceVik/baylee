//! Waterspout Elemental — {3}{U}{U} — Creature — Elemental
//! Oracle: Kicker {U} (You may pay an additional {U} as you cast this spell.)
//! Oracle: Flying
//! Oracle: When this creature enters, if it was kicked, return all other creatures to their owners' hands and you skip your next turn.
//! Set: PLS #38 — Planeshift | Scryfall ID: 425156e6-8eee-4bff-8f2f-86edd9a4f73b | Oracle ID: f86b8c03-9f1e-4d42-ac45-907c46e9e1c0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WATERSPOUT_ELEMENTAL,
    oracle_id = "f86b8c03-9f1e-4d42-ac45-907c46e9e1c0",
    scryfall_id = "425156e6-8eee-4bff-8f2f-86edd9a4f73b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Waterspout Elemental",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
