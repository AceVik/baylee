//! Collapsing Borders — {3}{R} — Enchantment
//! Oracle: Domain — At the beginning of each player's upkeep, that player gains 1 life for each basic land type among lands they control. Then this enchantment deals 3 damage to that player.
//! Set: INV #141 — Invasion | Scryfall ID: cc019633-788e-4095-9610-6c0a432f7656 | Oracle ID: 9fea3f42-643f-43c8-a9a9-4516dd72af5b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COLLAPSING_BORDERS,
    oracle_id = "9fea3f42-643f-43c8-a9a9-4516dd72af5b",
    scryfall_id = "cc019633-788e-4095-9610-6c0a432f7656",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Collapsing Borders",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
