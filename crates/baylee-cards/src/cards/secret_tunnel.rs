//! Secret Tunnel — (no cost) — Land — Cave
//! Oracle: This land can't be blocked.
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Two target creatures you control that share a creature type can't be blocked this turn.
//! Set: TLA #278 — Avatar: The Last Airbender | Scryfall ID: 2d39a0e1-6484-409c-ab05-5b276925a949 | Oracle ID: 632e2979-d88a-482e-9bb8-57b683c5310f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 995,
    oracle_id: "632e2979-d88a-482e-9bb8-57b683c5310f",
    scryfall_id: "2d39a0e1-6484-409c-ab05-5b276925a949",
    faces: &[
    face! {
        name: "Secret Tunnel",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
