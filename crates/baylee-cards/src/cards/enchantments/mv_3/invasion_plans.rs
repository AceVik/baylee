//! Invasion Plans — {2}{R} — Enchantment
//! Oracle: All creatures block each combat if able.
//! Oracle: The attacking player chooses how each creature blocks each combat.
//! Set: STH #89 — Stronghold | Scryfall ID: f52c2f83-9535-4ee5-9964-5cc01e617981 | Oracle ID: db9c0d94-9fa5-4f07-be48-23e19d2f3d68
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INVASION_PLANS,
    oracle_id = "db9c0d94-9fa5-4f07-be48-23e19d2f3d68",
    scryfall_id = "f52c2f83-9535-4ee5-9964-5cc01e617981",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Invasion Plans",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
