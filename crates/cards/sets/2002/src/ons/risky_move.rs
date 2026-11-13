//! Risky Move — {3}{R}{R}{R} — Enchantment
//! Oracle: At the beginning of each player's upkeep, that player gains control of this enchantment.
//! Oracle: When you gain control of this enchantment from another player, choose a creature you control and an opponent. Flip a coin. If you lose the flip, that opponent gains control of that creature.
//! Set: ONS #223 — Onslaught | Scryfall ID: 0b09315c-d6ff-4fdb-8774-c6402b45e959 | Oracle ID: 1a4c03ab-519a-4840-893a-769dfc5cf161
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RISKY_MOVE,
    oracle_id = "1a4c03ab-519a-4840-893a-769dfc5cf161",
    scryfall_id = "0b09315c-d6ff-4fdb-8774-c6402b45e959",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Risky Move",
        mana_cost = mana!("{3}{R}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
