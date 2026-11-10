//! Lashknife Barrier — {2}{W} — Enchantment
//! Oracle: When this enchantment enters, draw a card.
//! Oracle: If a source would deal damage to a creature you control, it deals that much damage minus 1 to that creature instead.
//! Set: PLS #9 — Planeshift | Scryfall ID: 2485c10d-de02-4be9-8119-afb2296e3317 | Oracle ID: 7e529372-cec7-40d7-a1ac-1dce1764f8b2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LASHKNIFE_BARRIER,
    oracle_id = "7e529372-cec7-40d7-a1ac-1dce1764f8b2",
    scryfall_id = "2485c10d-de02-4be9-8119-afb2296e3317",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Lashknife Barrier",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
