//! Branded Brawlers — {R} — Creature — Human Soldier
//! Oracle: This creature can't attack if defending player controls an untapped land.
//! Oracle: This creature can't block if you control an untapped land.
//! Set: PCY #84 — Prophecy | Scryfall ID: 90a48065-fbf1-4f2a-993e-7061057a4c45 | Oracle ID: e975a8b3-f817-42f3-84da-b03ebf95cf25
// IMPLEMENTED — conditional attack/block restrictions keyed on untapped lands.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static UNTAPPED_LAND: Filter = Filter::And(&[Filter::LAND, Filter::Untapped]);

card!(
    index = index::BRANDED_BRAWLERS,
    oracle_id = "e975a8b3-f817-42f3-84da-b03ebf95cf25",
    scryfall_id = "90a48065-fbf1-4f2a-993e-7061057a4c45",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Branded Brawlers",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::CantAttackIfDefenderControls(&UNTAPPED_LAND)
        ),
        static_ability!(
            Filter::This,
            Modifier::CantBlock(&Filter::Any),
            condition = Some(Condition::ControlCount(&UNTAPPED_LAND, 1)),
        ),
    ],
);
