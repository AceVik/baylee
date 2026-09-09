//! Hashep Oasis — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Pay 1 life: Add {G}.
//! Oracle: {1}{G}{G}, {T}, Sacrifice a Desert: Target creature gets +3/+3 until end of turn. Activate only as a sorcery.
//! Set: OTC #299 — Outlaws of Thunder Junction Commander | Scryfall ID: d18d5af2-ca2c-4a3a-9b67-e953b24b0718 | Oracle ID: eab70fff-6a9f-4f9f-89a2-b6910c199e46
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 604,
    oracle_id: "eab70fff-6a9f-4f9f-89a2-b6910c199e46",
    scryfall_id: "d18d5af2-ca2c-4a3a-9b67-e953b24b0718",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Hashep Oasis",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
