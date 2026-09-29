//! Deathgrip — {B}{B} — Enchantment
//! Oracle: {B}{B}: Counter target green spell.
//! Set: ME4 #75 — Masters Edition IV | Scryfall ID: fc86164d-24f1-4fad-a358-f61dbfd86bd9 | Oracle ID: 20ae75a7-14ca-4366-af0a-3f3f02159f3f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEATHGRIP,
    oracle_id = "20ae75a7-14ca-4366-af0a-3f3f02159f3f",
    scryfall_id = "fc86164d-24f1-4fad-a358-f61dbfd86bd9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Deathgrip",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
