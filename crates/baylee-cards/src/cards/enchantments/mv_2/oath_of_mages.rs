//! Oath of Mages — {1}{R} — Enchantment
//! Oracle: At the beginning of each player's upkeep, that player chooses target player who has more life than they do and is their opponent. The first player may have this enchantment deal 1 damage to the second player.
//! Set: EXO #90 — Exodus | Scryfall ID: ed8708d2-2c73-4da5-b6ff-41c083b59caa | Oracle ID: 2b749918-f425-4243-aabb-7350bb74403c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OATH_OF_MAGES,
    oracle_id = "2b749918-f425-4243-aabb-7350bb74403c",
    scryfall_id = "ed8708d2-2c73-4da5-b6ff-41c083b59caa",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Oath of Mages",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
