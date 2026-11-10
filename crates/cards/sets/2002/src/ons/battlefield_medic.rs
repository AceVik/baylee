//! Battlefield Medic — {1}{W} — Creature — Human Cleric
//! Oracle: {T}: Prevent the next X damage that would be dealt to target creature this turn, where X is the number of Clerics on the battlefield.
//! Set: ONS #9 — Onslaught | Scryfall ID: 9c444503-42a8-4952-819b-bbca89b06abc | Oracle ID: a600af0b-ea04-4813-b700-f51820868c86
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BATTLEFIELD_MEDIC,
    oracle_id = "a600af0b-ea04-4813-b700-f51820868c86",
    scryfall_id = "9c444503-42a8-4952-819b-bbca89b06abc",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Battlefield Medic",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
