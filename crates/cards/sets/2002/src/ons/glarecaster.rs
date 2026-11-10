//! Glarecaster — {4}{W}{W} — Creature — Bird Cleric
//! Oracle: Flying
//! Oracle: {5}{W}: The next time damage would be dealt to this creature and/or you this turn, that damage is dealt to any target instead.
//! Set: ONS #30 — Onslaught | Scryfall ID: 7e505e8e-51aa-4415-81e6-cf022279edb0 | Oracle ID: 9c058107-b2a1-4300-8d97-c697c248df96
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GLARECASTER,
    oracle_id = "9c058107-b2a1-4300-8d97-c697c248df96",
    scryfall_id = "7e505e8e-51aa-4415-81e6-cf022279edb0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Glarecaster",
        mana_cost = mana!("{4}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::CLERIC],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
