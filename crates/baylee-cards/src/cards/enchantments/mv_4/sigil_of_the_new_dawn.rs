//! Sigil of the New Dawn — {3}{W} — Enchantment
//! Oracle: Whenever a creature is put into your graveyard from the battlefield, you may pay {1}{W}. If you do, return that card to your hand.
//! Set: UMA #35 — Ultimate Masters | Scryfall ID: 64393586-42c6-4874-9473-85c63c7ed2ad | Oracle ID: cb029c0f-b08a-4952-b236-8166b9f17119
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SIGIL_OF_THE_NEW_DAWN,
    oracle_id = "cb029c0f-b08a-4952-b236-8166b9f17119",
    scryfall_id = "64393586-42c6-4874-9473-85c63c7ed2ad",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sigil of the New Dawn",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
