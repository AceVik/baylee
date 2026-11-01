//! Pyromancy — {2}{R}{R} — Enchantment
//! Oracle: {3}, Discard a card at random: This enchantment deals damage to any target equal to the mana value of the discarded card.
//! Set: ULG #88 — Urza's Legacy | Scryfall ID: 35a91a58-cc8b-47a3-8c53-43c32753a00d | Oracle ID: 71a176ce-cbe7-4cec-a2ca-58abf59ad964
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PYROMANCY,
    oracle_id = "71a176ce-cbe7-4cec-a2ca-58abf59ad964",
    scryfall_id = "35a91a58-cc8b-47a3-8c53-43c32753a00d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Pyromancy",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
