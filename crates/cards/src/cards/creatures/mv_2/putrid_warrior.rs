//! Putrid Warrior — {W}{B} — Creature — Zombie Soldier Warrior
//! Oracle: Whenever this creature deals damage, choose one —
//! Oracle: • Each player loses 1 life.
//! Oracle: • Each player gains 1 life.
//! Set: APC #117 — Apocalypse | Scryfall ID: 17fce298-3338-4f41-8156-ab6322951a76 | Oracle ID: 6ec24c24-7e87-4c12-b94e-3cd5b184beb5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PUTRID_WARRIOR,
    oracle_id = "6ec24c24-7e87-4c12-b94e-3cd5b184beb5",
    scryfall_id = "17fce298-3338-4f41-8156-ab6322951a76",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    faces = &[face!(
        name = "Putrid Warrior",
        mana_cost = mana!("{W}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::ZOMBIE,
            subtypes::creature::SOLDIER,
            subtypes::creature::WARRIOR
        ],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
