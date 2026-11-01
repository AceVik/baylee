//! Mistform Dreamer — {2}{U} — Creature — Illusion
//! Oracle: Flying
//! Oracle: {1}: This creature becomes the creature type of your choice until end of turn.
//! Set: ONS #93 — Onslaught | Scryfall ID: ff34e303-c94a-4f5f-b9f6-8d48e6aac383 | Oracle ID: 5c679c43-5990-4dcf-a36e-865c17bdea32
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISTFORM_DREAMER,
    oracle_id = "5c679c43-5990-4dcf-a36e-865c17bdea32",
    scryfall_id = "ff34e303-c94a-4f5f-b9f6-8d48e6aac383",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mistform Dreamer",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
