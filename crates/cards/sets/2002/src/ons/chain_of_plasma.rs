//! Chain of Plasma — {1}{R} — Instant
//! Oracle: Chain of Plasma deals 3 damage to any target. Then that player or that permanent's controller may discard a card. If the player does, they may copy this spell and may choose a new target for that copy.
//! Set: ONS #193 — Onslaught | Scryfall ID: f94aa774-9036-4016-8880-4bde2710cb90 | Oracle ID: 029d08cb-da4d-4f4e-b667-62712af63260
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHAIN_OF_PLASMA,
    oracle_id = "029d08cb-da4d-4f4e-b667-62712af63260",
    scryfall_id = "f94aa774-9036-4016-8880-4bde2710cb90",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Chain of Plasma",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
