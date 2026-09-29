//! Phantasmal Forces — {3}{U} — Creature — Illusion
//! Oracle: Flying
//! Oracle: At the beginning of your upkeep, sacrifice this creature unless you pay {U}.
//! Set: ME4 #55 — Masters Edition IV | Scryfall ID: 996fa830-7c09-4077-818d-24201617cf19 | Oracle ID: 06a158c6-7e36-49f8-a8e0-a7b7df5fd7ed
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHANTASMAL_FORCES,
    oracle_id = "06a158c6-7e36-49f8-a8e0-a7b7df5fd7ed",
    scryfall_id = "996fa830-7c09-4077-818d-24201617cf19",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Phantasmal Forces",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(4),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
