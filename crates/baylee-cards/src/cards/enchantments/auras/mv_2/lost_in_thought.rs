//! Lost in Thought — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't attack or block, and its activated abilities can't be activated. Its controller may exile three cards from their graveyard for that player to ignore this effect until end of turn.
//! Set: JUD #45 — Judgment | Scryfall ID: f5fb391a-2687-461d-b5ef-a494287ddb5d | Oracle ID: 820ba630-c543-491c-8c77-ada69b01c79d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LOST_IN_THOUGHT,
    oracle_id = "820ba630-c543-491c-8c77-ada69b01c79d",
    scryfall_id = "f5fb391a-2687-461d-b5ef-a494287ddb5d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Lost in Thought",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
