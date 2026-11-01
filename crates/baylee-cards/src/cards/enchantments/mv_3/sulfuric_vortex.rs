//! Sulfuric Vortex — {1}{R}{R} — Enchantment
//! Oracle: At the beginning of each player's upkeep, this enchantment deals 2 damage to that player.
//! Oracle: If a player would gain life, that player gains no life instead.
//! Set: DMR #144 — Dominaria Remastered | Scryfall ID: 3253a624-50cc-4a8a-9b0c-9902aa53775f | Oracle ID: 7652f328-e142-494b-a869-772ced10c26a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SULFURIC_VORTEX,
    oracle_id = "7652f328-e142-494b-a869-772ced10c26a",
    scryfall_id = "3253a624-50cc-4a8a-9b0c-9902aa53775f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Sulfuric Vortex",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
