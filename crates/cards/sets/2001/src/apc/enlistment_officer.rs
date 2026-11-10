//! Enlistment Officer — {3}{W} — Creature — Human Soldier
//! Oracle: First strike
//! Oracle: When this creature enters, reveal the top four cards of your library. Put all Soldier cards revealed this way into your hand and the rest on the bottom of your library in any order.
//! Set: APC #9 — Apocalypse | Scryfall ID: 38138bb4-25ea-4aaf-8b1c-e9e60678fc6b | Oracle ID: e9517fc3-d1bc-42f7-b052-9f629b302735
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ENLISTMENT_OFFICER,
    oracle_id = "e9517fc3-d1bc-42f7-b052-9f629b302735",
    scryfall_id = "38138bb4-25ea-4aaf-8b1c-e9e60678fc6b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Enlistment Officer",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
