//! Multani's Acolyte — {G}{G} — Creature — Elf
//! Oracle: Echo {G}{G} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: When this creature enters, draw a card.
//! Set: ULG #108 — Urza's Legacy | Scryfall ID: 4e5fdecb-bca0-48ea-b5bb-d0886c7d3316 | Oracle ID: d08725e6-ed9a-4c82-89d4-0dd037c376f6
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MULTANI_S_ACOLYTE,
    oracle_id = "d08725e6-ed9a-4c82-89d4-0dd037c376f6",
    scryfall_id = "4e5fdecb-bca0-48ea-b5bb-d0886c7d3316",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Multani's Acolyte",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[
        AbilityDef::Echo {
            cost: mana!("{G}{G}"),
        },
        triggered!(Trigger::ETB, &[Effect::draw(1)]),
    ],
);
