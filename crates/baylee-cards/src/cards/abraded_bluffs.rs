//! Abraded Bluffs — (no cost) — Land — Desert
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, it deals 1 damage to target opponent.
//! Oracle: {T}: Add {R} or {W}.
//! Set: OTJ #251 — Outlaws of Thunder Junction | Scryfall ID: 19e96521-b4ce-4a36-a887-200e05ccc804 | Oracle ID: ca7d093c-0533-493f-9ad3-8af30118fbfc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 199,
    oracle_id: "ca7d093c-0533-493f-9ad3-8af30118fbfc",
    scryfall_id: "19e96521-b4ce-4a36-a887-200e05ccc804",
    color_identity: ColorSet::from_slice(&[Color::Red, Color::White]),
    faces: &[
    face! {
        name: "Abraded Bluffs",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
