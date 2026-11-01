//! Veteran Explorer — {G} — Creature — Human Soldier Scout
//! Oracle: When this creature dies, each player may search their library for up to two basic land cards, put them onto the battlefield, then shuffle.
//! Set: 2XM #186 — Double Masters | Scryfall ID: 1a6c19cc-6469-4958-a547-1d0fe97c70e8 | Oracle ID: fa9677d2-1535-4c57-baa8-c0408e15da47
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VETERAN_EXPLORER,
    oracle_id = "fa9677d2-1535-4c57-baa8-c0408e15da47",
    scryfall_id = "1a6c19cc-6469-4958-a547-1d0fe97c70e8",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Veteran Explorer",
        mana_cost = mana!("{G}"),
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
