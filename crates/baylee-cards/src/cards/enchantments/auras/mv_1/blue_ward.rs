//! Blue Ward — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has protection from blue. This effect doesn't remove this Aura.
//! Set: 4ED #10 — Fourth Edition | Scryfall ID: de62c833-c66b-442e-99ed-99ccd8eca024 | Oracle ID: fc0bf1d0-46a2-4305-ab2e-466d79d60ab2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLUE_WARD,
    oracle_id = "fc0bf1d0-46a2-4305-ab2e-466d79d60ab2",
    scryfall_id = "de62c833-c66b-442e-99ed-99ccd8eca024",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Blue Ward",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
