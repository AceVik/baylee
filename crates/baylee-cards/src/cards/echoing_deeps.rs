//! Echoing Deeps — (no cost) — Land — Cave
//! Oracle: You may have this land enter tapped as a copy of any land card in a graveyard, except it's a Cave in addition to its other types.
//! Oracle: {T}: Add {C}.
//! Set: LCI #271 — The Lost Caverns of Ixalan | Scryfall ID: 244c06b3-532d-426e-8bee-ee9461d092a6 | Oracle ID: 2ef88214-f46d-473e-a55b-795a647e2f03
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 455,
    oracle_id: "2ef88214-f46d-473e-a55b-795a647e2f03",
    scryfall_id: "244c06b3-532d-426e-8bee-ee9461d092a6",
    faces: &[
    face! {
        name: "Echoing Deeps",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
