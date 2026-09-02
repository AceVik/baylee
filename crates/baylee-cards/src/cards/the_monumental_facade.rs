//! The Monumental Facade — (no cost) — Land — Sphere
//! Oracle: This land enters with two oil counters on it.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Remove an oil counter from this land: Put an oil counter on target artifact or creature you control. Activate only as a sorcery.
//! Set: ONE #255 — Phyrexia: All Will Be One | Scryfall ID: d6785057-0d06-4f91-b45f-c05f7c4e2b19 | Oracle ID: 63e8d282-d038-4a8c-a0cb-f51ddf87d8ea
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1177,
    oracle_id: "63e8d282-d038-4a8c-a0cb-f51ddf87d8ea",
    scryfall_id: "d6785057-0d06-4f91-b45f-c05f7c4e2b19",
    faces: &[
    face! {
        name: "The Monumental Facade",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::SPHERE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
