//! Chain of Vapor — {U} — Instant
//! Oracle: Return target nonland permanent to its owner's hand. Then that permanent's controller may sacrifice a land of their choice. If the player does, they may copy this spell and may choose a new target for that copy.
//! Set: C16 #84 — Commander 2016 | Scryfall ID: 38f741bf-13ee-4384-98ab-27833ebb9f6d | Oracle ID: 2167ac25-d042-4b48-b770-8b94acc1a965
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHAIN_OF_VAPOR,
    oracle_id = "2167ac25-d042-4b48-b770-8b94acc1a965",
    scryfall_id = "38f741bf-13ee-4384-98ab-27833ebb9f6d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Chain of Vapor",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
