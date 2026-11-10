//! Battle Strain — {1}{R} — Enchantment
//! Oracle: Whenever a creature blocks, this enchantment deals 1 damage to that creature's controller.
//! Set: ODY #177 — Odyssey | Scryfall ID: ee1be893-1287-40a3-81d6-271df3154b45 | Oracle ID: c4d49ecc-8650-4e91-9a39-a4a25213a5f7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BATTLE_STRAIN,
    oracle_id = "c4d49ecc-8650-4e91-9a39-a4a25213a5f7",
    scryfall_id = "ee1be893-1287-40a3-81d6-271df3154b45",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Battle Strain",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
