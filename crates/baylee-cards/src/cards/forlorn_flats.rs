//! Forlorn Flats — (no cost) — Land — Desert
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, it deals 1 damage to target opponent.
//! Oracle: {T}: Add {W} or {B}.
//! Set: OTJ #258 — Outlaws of Thunder Junction | Scryfall ID: 963c100e-4e12-438f-b5ae-14391406dff6 | Oracle ID: ebb3e2ff-2214-4e11-88fb-e0fa84288cf1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 514,
    oracle_id: "ebb3e2ff-2214-4e11-88fb-e0fa84288cf1",
    scryfall_id: "963c100e-4e12-438f-b5ae-14391406dff6",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::White]),
    faces: &[
    face! {
        name: "Forlorn Flats",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
