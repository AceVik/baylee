//! Soothsaying — {U} — Enchantment
//! Oracle: {3}{U}{U}: Shuffle your library.
//! Oracle: {X}: Look at the top X cards of your library, then put them back in any order.
//! Set: MMQ #104 — Mercadian Masques | Scryfall ID: def384ff-1b6f-4c4f-8151-3c72c29b63ce | Oracle ID: 517b702a-2c5c-40d7-825e-6c674019b298
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SOOTHSAYING,
    oracle_id = "517b702a-2c5c-40d7-825e-6c674019b298",
    scryfall_id = "def384ff-1b6f-4c4f-8151-3c72c29b63ce",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Soothsaying",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
