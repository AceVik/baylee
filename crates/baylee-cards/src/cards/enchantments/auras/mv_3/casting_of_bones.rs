//! Casting of Bones — {2}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When enchanted creature dies, draw three cards, then discard one of them.
//! Set: ALL #44a — Alliances | Scryfall ID: e823c295-b66e-41c3-bd77-1a13f95e69c3 | Oracle ID: 5a747256-4215-4334-98ab-0c2e4ed92e47
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CASTING_OF_BONES,
    oracle_id = "5a747256-4215-4334-98ab-0c2e4ed92e47",
    scryfall_id = "e823c295-b66e-41c3-bd77-1a13f95e69c3",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Casting of Bones",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
