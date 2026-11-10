//! Mana Echoes — {2}{R}{R} — Enchantment
//! Oracle: Whenever a creature enters, you may add an amount of {C} equal to the number of creatures you control that share a creature type with it.
//! Set: 2XM #136 — Double Masters | Scryfall ID: bd079929-fa58-4484-91b7-31305b87ee43 | Oracle ID: 369e90e5-a31b-49cc-b594-8a853f244be1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANA_ECHOES,
    oracle_id = "369e90e5-a31b-49cc-b594-8a853f244be1",
    scryfall_id = "bd079929-fa58-4484-91b7-31305b87ee43",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Mana Echoes",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
