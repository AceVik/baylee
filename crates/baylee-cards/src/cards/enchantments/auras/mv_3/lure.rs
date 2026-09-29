//! Lure — {1}{G}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: All creatures able to block enchanted creature do so.
//! Set: IMA #175 — Iconic Masters | Scryfall ID: 72c8336d-54cf-45af-a9ef-a1428facf91b | Oracle ID: 7a7425ba-4478-4bc4-855f-abf947ea4fa2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LURE,
    oracle_id = "7a7425ba-4478-4bc4-855f-abf947ea4fa2",
    scryfall_id = "72c8336d-54cf-45af-a9ef-a1428facf91b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Lure",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
