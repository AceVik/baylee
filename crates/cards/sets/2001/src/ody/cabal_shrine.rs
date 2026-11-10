//! Cabal Shrine — {1}{B}{B} — Enchantment
//! Oracle: Whenever a player casts a spell, that player discards X cards, where X is the number of cards in all graveyards with the same name as that spell.
//! Set: ODY #121 — Odyssey | Scryfall ID: dd376a52-5dfd-49f3-a520-537cd4527439 | Oracle ID: 3a1b4d0f-7051-4913-bced-bf7596b36668
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CABAL_SHRINE,
    oracle_id = "3a1b4d0f-7051-4913-bced-bf7596b36668",
    scryfall_id = "dd376a52-5dfd-49f3-a520-537cd4527439",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Cabal Shrine",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
