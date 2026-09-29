//! Ivory Cup — {1} — Artifact
//! Oracle: Whenever a player casts a white spell, you may pay {1}. If you do, you gain 1 life.
//! Set: 8ED #305 — Eighth Edition | Scryfall ID: fc4cd379-caba-485c-8c0f-e59804c387c6 | Oracle ID: 8e2017c3-057d-4e30-bd60-f486fddbc6ca
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::IVORY_CUP,
    oracle_id = "8e2017c3-057d-4e30-bd60-f486fddbc6ca",
    scryfall_id = "fc4cd379-caba-485c-8c0f-e59804c387c6",
    faces = &[face!(
        name = "Ivory Cup",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
