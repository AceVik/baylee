//! Sunglasses of Urza — {3} — Artifact
//! Oracle: You may spend white mana as though it were red mana.
//! Set: 4ED #347 — Fourth Edition | Scryfall ID: 6a225462-947c-49d7-81b2-91f875664dca | Oracle ID: eea64b1f-d6a9-4f72-8612-efab4b124b63
// PARTIAL — spending white mana as though it were red is not in the engine.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SUNGLASSES_OF_URZA,
    oracle_id = "eea64b1f-d6a9-4f72-8612-efab4b124b63",
    scryfall_id = "6a225462-947c-49d7-81b2-91f875664dca",
    coverage = Coverage::Partial("spending white mana as though it were red is not in the engine"),
    faces = &[face!(
        name = "Sunglasses of Urza",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    // NOT SUPPORTED: You may spend white mana as though it were red mana.
);
