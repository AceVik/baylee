//! Aven Soulgazer — {3}{W}{W} — Creature — Bird Cleric
//! Oracle: Flying
//! Oracle: {2}{W}: Look at target face-down creature.
//! Set: ONS #8 — Onslaught | Scryfall ID: 5189f152-f075-4090-97dd-b7686d813865 | Oracle ID: e7ac8c1d-d298-4d57-8e0e-c60c7e61b247
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AVEN_SOULGAZER,
    oracle_id = "e7ac8c1d-d298-4d57-8e0e-c60c7e61b247",
    scryfall_id = "5189f152-f075-4090-97dd-b7686d813865",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Aven Soulgazer",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::CLERIC],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
