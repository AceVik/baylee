//! Living Artifact — {G} — Enchantment — Aura
//! Oracle: Enchant artifact
//! Oracle: Whenever you're dealt damage, put that many vitality counters on this Aura.
//! Oracle: At the beginning of your upkeep, you may remove a vitality counter from this Aura. If you do, you gain 1 life.
//! Set: 5ED #311 — Fifth Edition | Scryfall ID: 8af097b1-9eae-4bd6-8ee6-582cae57e970 | Oracle ID: 4ff9af56-ac18-4966-9e48-183e1ca1c2d0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LIVING_ARTIFACT,
    oracle_id = "4ff9af56-ac18-4966-9e48-183e1ca1c2d0",
    scryfall_id = "8af097b1-9eae-4bd6-8ee6-582cae57e970",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Living Artifact",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
