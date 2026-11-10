//! Ground Seal — {1}{G} — Enchantment
//! Oracle: When this enchantment enters, draw a card.
//! Oracle: Cards in graveyards can't be the targets of spells or abilities.
//! Set: C18 #149 — Commander 2018 | Scryfall ID: c4e9995e-f26b-4638-b69d-a310f58f0331 | Oracle ID: 13f6f960-ef79-4f2c-8874-90fe9e77099e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GROUND_SEAL,
    oracle_id = "13f6f960-ef79-4f2c-8874-90fe9e77099e",
    scryfall_id = "c4e9995e-f26b-4638-b69d-a310f58f0331",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Ground Seal",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
