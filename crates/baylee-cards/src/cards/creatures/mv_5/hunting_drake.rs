//! Hunting Drake — {4}{U} — Creature — Drake
//! Oracle: Flying
//! Oracle: When this creature enters, put target red or green creature on top of its owner's library.
//! Set: PLS #27 — Planeshift | Scryfall ID: 5b0293a9-48fe-4018-bd25-3e02c227a3dd | Oracle ID: d5978943-52fc-4b57-9c1c-d802ade1b014
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HUNTING_DRAKE,
    oracle_id = "d5978943-52fc-4b57-9c1c-d802ade1b014",
    scryfall_id = "5b0293a9-48fe-4018-bd25-3e02c227a3dd",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Hunting Drake",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAKE],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
