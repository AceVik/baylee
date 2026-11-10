//! Branded Brawlers — {R} — Creature — Human Soldier
//! Oracle: This creature can't attack if defending player controls an untapped land.
//! Oracle: This creature can't block if you control an untapped land.
//! Set: PCY #84 — Prophecy | Scryfall ID: 90a48065-fbf1-4f2a-993e-7061057a4c45 | Oracle ID: e975a8b3-f817-42f3-84da-b03ebf95cf25
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BRANDED_BRAWLERS,
    oracle_id = "e975a8b3-f817-42f3-84da-b03ebf95cf25",
    scryfall_id = "90a48065-fbf1-4f2a-993e-7061057a4c45",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Branded Brawlers",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
