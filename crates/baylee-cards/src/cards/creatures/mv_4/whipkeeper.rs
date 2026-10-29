//! Whipkeeper — {2}{R}{R} — Creature — Dwarf
//! Oracle: {T}: This creature deals damage to target creature equal to the damage already dealt to it this turn.
//! Set: ODY #228 — Odyssey | Scryfall ID: 3d86db25-3ec1-42f3-8b02-2729d50dd201 | Oracle ID: 8c5a6c0e-53a3-4fc4-bb6e-c52f4d0e9490
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WHIPKEEPER,
    oracle_id = "8c5a6c0e-53a3-4fc4-bb6e-c52f4d0e9490",
    scryfall_id = "3d86db25-3ec1-42f3-8b02-2729d50dd201",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Whipkeeper",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DWARF],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
