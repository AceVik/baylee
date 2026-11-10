//! Ancient Ooze — {5}{G}{G} — Creature — Ooze
//! Oracle: Ancient Ooze's power and toughness are each equal to the total mana value of other creatures you control.
//! Set: SCG #112 — Scourge | Scryfall ID: 3b57b41c-f99c-4525-8541-f025b7e31974 | Oracle ID: 2076b2a7-6e07-4b6d-ae17-c1cfd0961022
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ANCIENT_OOZE,
    oracle_id = "2076b2a7-6e07-4b6d-ae17-c1cfd0961022",
    scryfall_id = "3b57b41c-f99c-4525-8541-f025b7e31974",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Ancient Ooze",
        mana_cost = mana!("{5}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OOZE],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
