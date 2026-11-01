//! Traveling Plague — {3}{B}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: At the beginning of each upkeep, put a plague counter on this Aura.
//! Oracle: Enchanted creature gets -1/-1 for each plague counter on this Aura.
//! Oracle: When enchanted creature leaves the battlefield, that creature's controller returns this Aura from its owner's graveyard to the battlefield.
//! Set: ODY #166 — Odyssey | Scryfall ID: d4b6413d-68d3-4097-9bb3-873df4900e4c | Oracle ID: 371e8e29-f569-4b23-bae4-cf89e5b09fd3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TRAVELING_PLAGUE,
    oracle_id = "371e8e29-f569-4b23-bae4-cf89e5b09fd3",
    scryfall_id = "d4b6413d-68d3-4097-9bb3-873df4900e4c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Traveling Plague",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
