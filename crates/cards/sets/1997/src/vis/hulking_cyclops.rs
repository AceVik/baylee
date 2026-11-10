//! Hulking Cyclops — {3}{R}{R} — Creature — Cyclops
//! Oracle: This creature can't block.
//! Set: 8ED #195 — Eighth Edition | Scryfall ID: dc1a5428-9bb4-4e2b-ba94-fccd321ce537 | Oracle ID: 04936918-7ef3-42fa-b0cd-87d18cb69d2c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HULKING_CYCLOPS,
    oracle_id = "04936918-7ef3-42fa-b0cd-87d18cb69d2c",
    scryfall_id = "dc1a5428-9bb4-4e2b-ba94-fccd321ce537",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hulking Cyclops",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CYCLOPS],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CantBlock(&Filter::Any)
    )],
);
