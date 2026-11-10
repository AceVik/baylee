//! Harsh Judgment — {2}{W}{W} — Enchantment
//! Oracle: As this enchantment enters, choose a color.
//! Oracle: If an instant or sorcery spell of the chosen color would deal damage to you, it deals that damage to its controller instead.
//! Set: INV #19 — Invasion | Scryfall ID: 34c78dee-ab45-4638-b89a-10686145b19a | Oracle ID: 1fa272bf-8759-4750-b40b-8e2f6972d570
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HARSH_JUDGMENT,
    oracle_id = "1fa272bf-8759-4750-b40b-8e2f6972d570",
    scryfall_id = "34c78dee-ab45-4638-b89a-10686145b19a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Harsh Judgment",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
