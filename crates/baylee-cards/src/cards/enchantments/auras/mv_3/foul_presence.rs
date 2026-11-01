//! Foul Presence — {2}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets -1/-1 and has "{T}: Target creature gets -1/-1 until end of turn."
//! Set: APC #39 — Apocalypse | Scryfall ID: 7c5a6fa8-d422-4e56-9e7b-2ff2fc8aecfe | Oracle ID: 16f3a09f-e29b-4fb5-95ed-d76ad550c2ad
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FOUL_PRESENCE,
    oracle_id = "16f3a09f-e29b-4fb5-95ed-d76ad550c2ad",
    scryfall_id = "7c5a6fa8-d422-4e56-9e7b-2ff2fc8aecfe",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Foul Presence",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
