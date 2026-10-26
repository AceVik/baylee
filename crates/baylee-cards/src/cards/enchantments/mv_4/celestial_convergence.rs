//! Celestial Convergence — {2}{W}{W} — Enchantment
//! Oracle: This enchantment enters with seven omen counters on it.
//! Oracle: At the beginning of your upkeep, remove an omen counter from this enchantment. If there are no omen counters on this enchantment, the player with the highest life total wins the game. If two or more players are tied for highest life total, the game is a draw.
//! Set: PCY #5 — Prophecy | Scryfall ID: e8e5c9ca-b453-488b-8702-fc74907a8335 | Oracle ID: 92f0677f-64d8-4c83-abad-ffa2a37f88c1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CELESTIAL_CONVERGENCE,
    oracle_id = "92f0677f-64d8-4c83-abad-ffa2a37f88c1",
    scryfall_id = "e8e5c9ca-b453-488b-8702-fc74907a8335",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Celestial Convergence",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
