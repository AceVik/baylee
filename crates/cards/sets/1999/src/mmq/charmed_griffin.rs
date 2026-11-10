//! Charmed Griffin — {3}{W} — Creature — Griffin
//! Oracle: Flying
//! Oracle: When this creature enters, each other player may put an artifact or enchantment card onto the battlefield from their hand.
//! Set: MMQ #7 — Mercadian Masques | Scryfall ID: 66d36960-3c78-4032-9325-8002b2a48503 | Oracle ID: 6a58f2a6-ff5e-4823-b3a7-49df14d64ffe
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHARMED_GRIFFIN,
    oracle_id = "6a58f2a6-ff5e-4823-b3a7-49df14d64ffe",
    scryfall_id = "66d36960-3c78-4032-9325-8002b2a48503",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Charmed Griffin",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GRIFFIN],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
