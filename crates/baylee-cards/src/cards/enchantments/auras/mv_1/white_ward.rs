//! White Ward — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has protection from white. This effect doesn't remove this Aura.
//! Set: 4ED #57 — Fourth Edition | Scryfall ID: 2d3c467f-0e33-43e8-b108-0ea00d6adcc2 | Oracle ID: 860468b3-625a-4663-a5ae-336ae10fc7d0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WHITE_WARD,
    oracle_id = "860468b3-625a-4663-a5ae-336ae10fc7d0",
    scryfall_id = "2d3c467f-0e33-43e8-b108-0ea00d6adcc2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "White Ward",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
