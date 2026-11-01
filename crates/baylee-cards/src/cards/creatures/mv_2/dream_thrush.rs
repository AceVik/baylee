//! Dream Thrush — {1}{U} — Creature — Bird
//! Oracle: Flying
//! Oracle: {T}: Target land becomes the basic land type of your choice until end of turn.
//! Set: INV #53 — Invasion | Scryfall ID: 258217df-ae88-4d93-895a-3fd242baacd1 | Oracle ID: 111f3f1c-4387-4e61-8bff-6ebe3b7b9d9f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DREAM_THRUSH,
    oracle_id = "111f3f1c-4387-4e61-8bff-6ebe3b7b9d9f",
    scryfall_id = "258217df-ae88-4d93-895a-3fd242baacd1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Dream Thrush",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
