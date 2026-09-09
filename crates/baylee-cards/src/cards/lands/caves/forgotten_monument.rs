//! Forgotten Monument — (no cost) — Land — Cave
//! Oracle: {T}: Add {C}.
//! Oracle: Other Caves you control have "{T}, Pay 1 life: Add one mana of any color."
//! Set: LCI #272 — The Lost Caverns of Ixalan | Scryfall ID: de8c1c02-e533-46b2-a3eb-91dff561854b | Oracle ID: 71393988-ad6f-43fd-9978-c0de15ae8e87
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 513,
    oracle_id: "71393988-ad6f-43fd-9978-c0de15ae8e87",
    scryfall_id: "de8c1c02-e533-46b2-a3eb-91dff561854b",
    faces: &[
    face! {
        name: "Forgotten Monument",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
