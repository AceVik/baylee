//! Alpha Status — {2}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +2/+2 for each other creature on the battlefield that shares a creature type with it.
//! Set: SCG #110 — Scourge | Scryfall ID: fd210c45-57f3-4d7d-93ba-81fe4298ade3 | Oracle ID: 839fb071-6585-4f3e-a13e-1c14779b401a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ALPHA_STATUS,
    oracle_id = "839fb071-6585-4f3e-a13e-1c14779b401a",
    scryfall_id = "fd210c45-57f3-4d7d-93ba-81fe4298ade3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Alpha Status",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
