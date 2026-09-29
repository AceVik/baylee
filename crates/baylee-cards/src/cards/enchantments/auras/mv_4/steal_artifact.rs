//! Steal Artifact — {2}{U}{U} — Enchantment — Aura
//! Oracle: Enchant artifact
//! Oracle: You control enchanted artifact.
//! Set: 8ED #103 — Eighth Edition | Scryfall ID: 810f874e-98e1-402b-b48e-14c3e2a624f0 | Oracle ID: cd8ae9f2-edac-473a-8846-c08219e617c3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STEAL_ARTIFACT,
    oracle_id = "cd8ae9f2-edac-473a-8846-c08219e617c3",
    scryfall_id = "810f874e-98e1-402b-b48e-14c3e2a624f0",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Steal Artifact",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
