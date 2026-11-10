//! Shade's Form — {1}{B}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has "{B}: This creature gets +1/+1 until end of turn."
//! Oracle: When enchanted creature dies, return that card to the battlefield under your control.
//! Set: TOR #81 — Torment | Scryfall ID: 449baad3-9a4a-4f5f-9e9f-02cc97ab71e6 | Oracle ID: 682a6d77-e8cd-4bf2-b777-7793101e0fd3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHADE_S_FORM,
    oracle_id = "682a6d77-e8cd-4bf2-b777-7793101e0fd3",
    scryfall_id = "449baad3-9a4a-4f5f-9e9f-02cc97ab71e6",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Shade's Form",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
