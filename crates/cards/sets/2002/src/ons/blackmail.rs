//! Blackmail — {B} — Sorcery
//! Oracle: Target player reveals three cards from their hand and you choose one of them. That player discards that card.
//! Set: 9ED #115 — Ninth Edition | Scryfall ID: 053e7c76-16b7-44eb-9612-a6800ffb35f8 | Oracle ID: 2dc6da5d-c4d1-4f2c-8f46-11a935bc0044
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLACKMAIL,
    oracle_id = "2dc6da5d-c4d1-4f2c-8f46-11a935bc0044",
    scryfall_id = "053e7c76-16b7-44eb-9612-a6800ffb35f8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Blackmail",
        mana_cost = mana!("{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
