//! Righteous Indignation — {2}{W} — Enchantment
//! Oracle: Whenever a creature blocks a black or red creature, the blocking creature gets +1/+1 until end of turn.
//! Set: MMQ #46 — Mercadian Masques | Scryfall ID: c1fb6335-cfd8-438c-b936-09b850d61b28 | Oracle ID: 97409c60-537e-4eeb-9d63-be0fde7e8284
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RIGHTEOUS_INDIGNATION,
    oracle_id = "97409c60-537e-4eeb-9d63-be0fde7e8284",
    scryfall_id = "c1fb6335-cfd8-438c-b936-09b850d61b28",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Righteous Indignation",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
