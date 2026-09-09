//! Glimmerpost — (no cost) — Land — Locus
//! Oracle: When this land enters, you gain 1 life for each Locus on the battlefield.
//! Oracle: {T}: Add {C}.
//! Set: SOM #227 — Scars of Mirrodin | Scryfall ID: 8b63efb6-249c-4f57-9af1-baffe938520c | Oracle ID: 92c9aad6-35ec-425d-be7d-393328992820
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 554,
    oracle_id: "92c9aad6-35ec-425d-be7d-393328992820",
    scryfall_id: "8b63efb6-249c-4f57-9af1-baffe938520c",
    faces: &[
    face! {
        name: "Glimmerpost",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::LOCUS],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
