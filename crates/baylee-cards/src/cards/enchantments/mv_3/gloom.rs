//! Gloom — {2}{B} — Enchantment
//! Oracle: White spells cost {3} more to cast.
//! Oracle: Activated abilities of white enchantments cost {3} more to activate.
//! Set: ME4 #83 — Masters Edition IV | Scryfall ID: 0d26c559-ad06-4f78-b7b6-e658be8c7bdb | Oracle ID: 4d022f53-b1fb-4071-afcc-0af3214fe604
// PARTIAL — costs raised for white spells and white enchantments' abilities are
// not in the engine; it does nothing.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GLOOM,
    oracle_id = "4d022f53-b1fb-4071-afcc-0af3214fe604",
    scryfall_id = "0d26c559-ad06-4f78-b7b6-e658be8c7bdb",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "costs raised for white spells and white enchantments' abilities are not in the engine; it does nothing"
    ),
    faces = &[face!(
        name = "Gloom",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        // NOT SUPPORTED: White spells cost {3} more to cast.
        // NOT SUPPORTED: Activated abilities of white enchantments cost {3} more to
        // activate.
    ],
);
