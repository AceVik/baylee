//! Cornered Market — {2}{W} — Enchantment
//! Oracle: Players can't cast spells with the same name as a nontoken permanent.
//! Oracle: Players can't play nonbasic lands with the same name as a nontoken permanent.
//! Set: MMQ #14 — Mercadian Masques | Scryfall ID: 0d4f3c1d-d25e-4263-ab2b-19534c852678 | Oracle ID: 1784bbaa-67b1-4bd1-bf77-53da2bfbf7fc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CORNERED_MARKET,
    oracle_id = "1784bbaa-67b1-4bd1-bf77-53da2bfbf7fc",
    scryfall_id = "0d4f3c1d-d25e-4263-ab2b-19534c852678",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Cornered Market",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
