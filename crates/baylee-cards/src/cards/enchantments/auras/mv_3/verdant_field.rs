//! Verdant Field — {2}{G} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Enchanted land has "{T}: Target creature gets +1/+1 until end of turn."
//! Set: PCY #130 — Prophecy | Scryfall ID: d123da53-9fd3-492b-beb7-76d1c0f5e4f6 | Oracle ID: 4d59fd2b-7fc6-49a7-8938-d3f08902a5a8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VERDANT_FIELD,
    oracle_id = "4d59fd2b-7fc6-49a7-8938-d3f08902a5a8",
    scryfall_id = "d123da53-9fd3-492b-beb7-76d1c0f5e4f6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Verdant Field",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
