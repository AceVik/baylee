//! Chain of Smog — {1}{B} — Sorcery
//! Oracle: Target player discards two cards. That player may copy this spell and may choose a new target for that copy.
//! Set: ONS #132 — Onslaught | Scryfall ID: 6bfe64f9-8b03-41f6-a47b-fade397ad9d1 | Oracle ID: ea14c26b-bf2f-48b4-b879-6e63069ded1f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHAIN_OF_SMOG,
    oracle_id = "ea14c26b-bf2f-48b4-b879-6e63069ded1f",
    scryfall_id = "6bfe64f9-8b03-41f6-a47b-fade397ad9d1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Chain of Smog",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
