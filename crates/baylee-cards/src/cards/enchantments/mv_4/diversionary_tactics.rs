//! Diversionary Tactics — {3}{W} — Enchantment
//! Oracle: Tap two untapped creatures you control: Tap target creature.
//! Set: APC #7 — Apocalypse | Scryfall ID: 1e5061e4-a76d-4a7c-b196-96c81f94e0e5 | Oracle ID: 880367e4-50b9-49d0-b596-d462d9b42855
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DIVERSIONARY_TACTICS,
    oracle_id = "880367e4-50b9-49d0-b596-d462d9b42855",
    scryfall_id = "1e5061e4-a76d-4a7c-b196-96c81f94e0e5",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Diversionary Tactics",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
