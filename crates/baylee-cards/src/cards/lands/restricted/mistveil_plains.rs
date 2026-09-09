//! Mistveil Plains — (no cost) — Land — Plains
//! Oracle: ({T}: Add {W}.)
//! Oracle: This land enters tapped.
//! Oracle: {W}, {T}: Put target card from your graveyard on the bottom of your library. Activate only if you control two or more white permanents.
//! Set: SOC #386 — Secrets of Strixhaven Commander | Scryfall ID: c53028dd-efb5-486c-a7d3-45f2f6050c1d | Oracle ID: bb5c1817-ac22-4779-9005-251bc354f181
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 772,
    oracle_id: "bb5c1817-ac22-4779-9005-251bc354f181",
    scryfall_id: "c53028dd-efb5-486c-a7d3-45f2f6050c1d",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Mistveil Plains",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::PLAINS],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
