//! Mistform Stalker — {1}{U} — Creature — Illusion
//! Oracle: {1}: This creature becomes the creature type of your choice until end of turn.
//! Oracle: {2}{U}{U}: This creature gets +2/+2 and gains flying until end of turn.
//! Set: ONS #98 — Onslaught | Scryfall ID: 9e80d109-b73f-4b5d-b9e4-534e8d69633f | Oracle ID: 0f9c4b56-7ad7-4262-8079-3eee5f0cdd3b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISTFORM_STALKER,
    oracle_id = "0f9c4b56-7ad7-4262-8079-3eee5f0cdd3b",
    scryfall_id = "9e80d109-b73f-4b5d-b9e4-534e8d69633f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mistform Stalker",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
