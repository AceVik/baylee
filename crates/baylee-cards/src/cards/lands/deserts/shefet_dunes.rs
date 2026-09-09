//! Shefet Dunes — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Pay 1 life: Add {W}.
//! Oracle: {2}{W}{W}, {T}, Sacrifice a Desert: Creatures you control get +1/+1 until end of turn. Activate only as a sorcery.
//! Set: OTC #318 — Outlaws of Thunder Junction Commander | Scryfall ID: c9b0a526-73b0-4501-80a3-f16dbef9cdfd | Oracle ID: 8305715e-f711-47d6-8efe-d0efe4ced418
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1013,
    oracle_id: "8305715e-f711-47d6-8efe-d0efe4ced418",
    scryfall_id: "c9b0a526-73b0-4501-80a3-f16dbef9cdfd",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Shefet Dunes",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
