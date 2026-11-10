//! Common Cause — {2}{W} — Enchantment
//! Oracle: Nonartifact creatures get +2/+2 as long as they all share a color.
//! Set: MMQ #13 — Mercadian Masques | Scryfall ID: eae4c25f-2005-4ac0-a5f0-2fc250520995 | Oracle ID: d1dac139-de12-46ba-94b1-42e107c2c95f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COMMON_CAUSE,
    oracle_id = "d1dac139-de12-46ba-94b1-42e107c2c95f",
    scryfall_id = "eae4c25f-2005-4ac0-a5f0-2fc250520995",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Common Cause",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
