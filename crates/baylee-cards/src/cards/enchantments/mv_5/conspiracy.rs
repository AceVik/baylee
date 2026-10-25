//! Conspiracy — {3}{B}{B} — Enchantment
//! Oracle: As this enchantment enters, choose a creature type.
//! Oracle: Creatures you control are the chosen type. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.
//! Set: TSB #39 — Time Spiral Timeshifted | Scryfall ID: 8bbdb402-0e22-4f83-987a-9d0268a82f10 | Oracle ID: 775d284d-0884-4ae3-a1b5-2b2a50a895b4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CONSPIRACY,
    oracle_id = "775d284d-0884-4ae3-a1b5-2b2a50a895b4",
    scryfall_id = "8bbdb402-0e22-4f83-987a-9d0268a82f10",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Conspiracy",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
