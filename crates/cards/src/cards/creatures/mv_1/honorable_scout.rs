//! Honorable Scout — {W} — Creature — Human Soldier Scout
//! Oracle: When this creature enters, you gain 2 life for each black and/or red creature target opponent controls.
//! Set: PLS #8 — Planeshift | Scryfall ID: bd311758-0352-4b7d-a24f-7f3f2b5d7b0f | Oracle ID: 16345f75-f33e-4a31-adb1-d3563358fcec
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HONORABLE_SCOUT,
    oracle_id = "16345f75-f33e-4a31-adb1-d3563358fcec",
    scryfall_id = "bd311758-0352-4b7d-a24f-7f3f2b5d7b0f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Honorable Scout",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::SOLDIER,
            subtypes::creature::SCOUT
        ],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
