//! Unnatural Selection — {1}{U} — Enchantment
//! Oracle: {1}: Choose a creature type other than Wall. Target creature becomes that type until end of turn.
//! Set: APC #32 — Apocalypse | Scryfall ID: c575e2cb-3990-4c73-b81c-e16311ec6bbb | Oracle ID: 0be10525-09dc-4970-b495-2db7d4d3c3e7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UNNATURAL_SELECTION,
    oracle_id = "0be10525-09dc-4970-b495-2db7d4d3c3e7",
    scryfall_id = "c575e2cb-3990-4c73-b81c-e16311ec6bbb",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Unnatural Selection",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
