//! Ramunap Ruins — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Pay 1 life: Add {R}.
//! Oracle: {2}{R}{R}, {T}, Sacrifice a Desert: This land deals 2 damage to each opponent.
//! Set: OTC #311 — Outlaws of Thunder Junction Commander | Scryfall ID: 727beb1f-1445-4398-970c-e31819d54bc6 | Oracle ID: d0d35864-1edc-4af1-9b89-3d7e94908011
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 895,
    oracle_id: "d0d35864-1edc-4af1-9b89-3d7e94908011",
    scryfall_id: "727beb1f-1445-4398-970c-e31819d54bc6",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    faces: &[
    face! {
        name: "Ramunap Ruins",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
