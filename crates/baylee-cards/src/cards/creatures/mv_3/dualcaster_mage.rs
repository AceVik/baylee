//! Dualcaster Mage — {1}{R}{R} — Creature — Human Wizard
//! Oracle: Flash
//! Oracle: When this creature enters, copy target instant or sorcery spell. You may choose new targets for the copy.
//! Set: C21 #165 — Commander 2021 | Scryfall ID: defcc4a3-40e0-4f5d-b23c-6cd6a614abc1 | Oracle ID: 8eb7c0a5-6190-40de-b473-2d1daa3bbe28
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DUALCASTER_MAGE,
    oracle_id = "8eb7c0a5-6190-40de-b473-2d1daa3bbe28",
    scryfall_id = "defcc4a3-40e0-4f5d-b23c-6cd6a614abc1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Dualcaster Mage",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
