//! Baldur's Gate — (no cost) — Legendary Land — Gate
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Add X mana of any one color, where X is the number of other Gates you control.
//! Set: CLB #345 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 2436aa14-9200-4295-8041-b682cf3c4216 | Oracle ID: da307ea2-4df7-4d6b-be0f-9dc6ac93db61
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 254,
    oracle_id: "da307ea2-4df7-4d6b-be0f-9dc6ac93db61",
    scryfall_id: "2436aa14-9200-4295-8041-b682cf3c4216",
    faces: &[
    face! {
        name: "Baldur's Gate",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::land::GATE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
