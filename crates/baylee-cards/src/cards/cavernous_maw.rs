//! Cavernous Maw — (no cost) — Land — Cave
//! Oracle: {T}: Add {C}.
//! Oracle: {2}: This land becomes a 3/3 Elemental creature until end of turn. It's still a Cave land. Activate only if the number of other Caves you control plus the number of Cave cards in your graveyard is three or greater.
//! Set: LCI #270 — The Lost Caverns of Ixalan | Scryfall ID: 2a51ebf6-a465-42e2-82b7-d2cb928ca632 | Oracle ID: 952ab8fe-f7d3-4673-89de-8c6d3f8a081f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 343,
    oracle_id: "952ab8fe-f7d3-4673-89de-8c6d3f8a081f",
    scryfall_id: "2a51ebf6-a465-42e2-82b7-d2cb928ca632",
    faces: &[
    face! {
        name: "Cavernous Maw",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
