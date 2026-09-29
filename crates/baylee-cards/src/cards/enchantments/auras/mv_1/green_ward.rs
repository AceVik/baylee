//! Green Ward — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has protection from green. This effect doesn't remove this Aura.
//! Set: 4ED #27 — Fourth Edition | Scryfall ID: b7548dc7-11dc-4611-85a5-71bdb48d8a62 | Oracle ID: 727ab7f2-741e-4442-b5cb-e3032549fa87
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GREEN_WARD,
    oracle_id = "727ab7f2-741e-4442-b5cb-e3032549fa87",
    scryfall_id = "b7548dc7-11dc-4611-85a5-71bdb48d8a62",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Green Ward",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
