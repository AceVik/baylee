//! Coalition Flag — {W} — Enchantment — Aura
//! Oracle: Enchant creature you control
//! Oracle: Enchanted creature is a Flagbearer.
//! Oracle: While an opponent is choosing targets as part of casting a spell they control or activating an ability they control, that player must choose at least one Flagbearer on the battlefield if able.
//! Set: APC #2 — Apocalypse | Scryfall ID: 0e417461-a230-4548-bcc1-71377487f21b | Oracle ID: b45a0a2f-39fd-47b9-bc9d-3d4321b377d3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::COALITION_FLAG,
    oracle_id = "b45a0a2f-39fd-47b9-bc9d-3d4321b377d3",
    scryfall_id = "0e417461-a230-4548-bcc1-71377487f21b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Coalition Flag",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
