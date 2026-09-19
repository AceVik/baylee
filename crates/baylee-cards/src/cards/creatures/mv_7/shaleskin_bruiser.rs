//! Shaleskin Bruiser — {6}{R} — Creature — Beast
//! Oracle: Trample
//! Oracle: Whenever this creature attacks, it gets +3/+0 until end of turn for each other attacking Beast.
//! Set: ONS #226 — Onslaught | Scryfall ID: fc2de8a4-0d84-4f7c-bbe4-3a31172186ab | Oracle ID: b90e370a-5080-485e-a957-93d5f60e6cdb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHALESKIN_BRUISER,
    oracle_id = "b90e370a-5080-485e-a957-93d5f60e6cdb",
    scryfall_id = "fc2de8a4-0d84-4f7c-bbe4-3a31172186ab",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Shaleskin Bruiser",
        mana_cost = mana!("{6}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
