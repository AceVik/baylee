//! Pemmin's Aura — {1}{U}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: {U}: Untap enchanted creature.
//! Oracle: {U}: Enchanted creature gains flying until end of turn.
//! Oracle: {U}: Enchanted creature gains shroud until end of turn. (It can't be the target of spells or abilities.)
//! Oracle: {1}: Enchanted creature gets +1/-1 or -1/+1 until end of turn.
//! Set: SCG #45 — Scourge | Scryfall ID: 9fb3e38b-086e-4fbc-b7b1-8564c18276d7 | Oracle ID: ada2f380-bca7-4d64-9587-bab11c776ba8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PEMMIN_S_AURA,
    oracle_id = "ada2f380-bca7-4d64-9587-bab11c776ba8",
    scryfall_id = "9fb3e38b-086e-4fbc-b7b1-8564c18276d7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Pemmin's Aura",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
