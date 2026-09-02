//! Pit of Offerings — (no cost) — Land — Cave
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, exile up to three target cards from graveyards.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any of the exiled cards' colors.
//! Set: LCI #278 — The Lost Caverns of Ixalan | Scryfall ID: bc7d3957-b483-4a1f-a244-293c90032f5e | Oracle ID: 044d2788-6daa-4849-a813-1f577eef9295
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 864,
    oracle_id: "044d2788-6daa-4849-a813-1f577eef9295",
    scryfall_id: "bc7d3957-b483-4a1f-a244-293c90032f5e",
    faces: &[
    face! {
        name: "Pit of Offerings",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
