//! Dress Down — {1}{U} — Enchantment
//! Oracle: Flash
//! Oracle: When this enchantment enters, draw a card.
//! Oracle: Creatures lose all abilities.
//! Oracle: At the beginning of the end step, sacrifice this enchantment.
//! Set: MH2 #39 — Modern Horizons 2 | Scryfall ID: 04f9f061-67b8-4427-9fcb-b3ccfee8fc5d | Oracle ID: c1cbbda0-e02b-4b9c-8669-ea87ca629520
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DRESS_DOWN,
    oracle_id = "c1cbbda0-e02b-4b9c-8669-ea87ca629520",
    scryfall_id = "04f9f061-67b8-4427-9fcb-b3ccfee8fc5d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Dress Down",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
