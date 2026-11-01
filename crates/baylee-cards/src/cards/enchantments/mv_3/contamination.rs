//! Contamination — {2}{B} — Enchantment
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless you sacrifice a creature.
//! Oracle: If a land is tapped for mana, it produces {B} instead of any other type and amount.
//! Set: USG #123 — Urza's Saga | Scryfall ID: 86067dfe-65c3-4c96-bccd-b3915d6663f9 | Oracle ID: 4013b4c2-c9ed-4d14-90f0-97214ec1fded
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CONTAMINATION,
    oracle_id = "4013b4c2-c9ed-4d14-90f0-97214ec1fded",
    scryfall_id = "86067dfe-65c3-4c96-bccd-b3915d6663f9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Contamination",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
