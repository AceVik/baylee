//! Dwarven Shrine — {1}{R}{R} — Enchantment
//! Oracle: Whenever a player casts a spell, this enchantment deals X damage to that player, where X is twice the number of cards in all graveyards with the same name as that spell.
//! Set: ODY #187 — Odyssey | Scryfall ID: 85197997-5e1a-46ce-8f0d-6da5ce297baf | Oracle ID: ecf613a7-a3df-4da6-b3db-4626b5ae8df6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DWARVEN_SHRINE,
    oracle_id = "ecf613a7-a3df-4da6-b3db-4626b5ae8df6",
    scryfall_id = "85197997-5e1a-46ce-8f0d-6da5ce297baf",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Dwarven Shrine",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
