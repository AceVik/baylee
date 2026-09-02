//! Urza's Cave — (no cost) — Land — Urza's Cave
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}, Sacrifice this land: Search your library for a land card, put it onto the battlefield tapped, then shuffle.
//! Set: MH3 #234 — Modern Horizons 3 | Scryfall ID: 926916ed-2f22-4ba9-9427-194886ad6c1e | Oracle ID: 4474ecee-0ec3-409b-90df-738d9313fe3c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1266,
    oracle_id: "4474ecee-0ec3-409b-90df-738d9313fe3c",
    scryfall_id: "926916ed-2f22-4ba9-9427-194886ad6c1e",
    faces: &[
    face! {
        name: "Urza's Cave",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::URZA_S, subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
