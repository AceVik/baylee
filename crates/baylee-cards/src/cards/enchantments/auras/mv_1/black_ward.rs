//! Black Ward — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has protection from black. This effect doesn't remove this Aura.
//! Set: 4ED #8 — Fourth Edition | Scryfall ID: 218b1327-5f76-4ee2-a93d-d2ba412043c2 | Oracle ID: 7861ac9b-3024-4935-804c-2ca4c5a46bf4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLACK_WARD,
    oracle_id = "7861ac9b-3024-4935-804c-2ca4c5a46bf4",
    scryfall_id = "218b1327-5f76-4ee2-a93d-d2ba412043c2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Black Ward",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
