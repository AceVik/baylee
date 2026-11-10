//! Earnest Fellowship — {1}{W} — Enchantment
//! Oracle: Each creature has protection from each of its colors.
//! Set: ODY #21 — Odyssey | Scryfall ID: 66392169-5c6f-46bf-b0df-5670e40aecd9 | Oracle ID: 26f87a39-29ed-4649-b5d8-204f6a40a41d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EARNEST_FELLOWSHIP,
    oracle_id = "26f87a39-29ed-4649-b5d8-204f6a40a41d",
    scryfall_id = "66392169-5c6f-46bf-b0df-5670e40aecd9",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Earnest Fellowship",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
