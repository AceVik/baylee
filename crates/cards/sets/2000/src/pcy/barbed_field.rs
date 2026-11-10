//! Barbed Field — {2}{R}{R} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Enchanted land has "{T}: This land deals 1 damage to any target."
//! Set: PCY #83 — Prophecy | Scryfall ID: 1c76db48-6f05-49c3-a49c-587c0a8a3613 | Oracle ID: 45222123-f5e0-48ff-b80c-858727d155d7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BARBED_FIELD,
    oracle_id = "45222123-f5e0-48ff-b80c-858727d155d7",
    scryfall_id = "1c76db48-6f05-49c3-a49c-587c0a8a3613",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Barbed Field",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
