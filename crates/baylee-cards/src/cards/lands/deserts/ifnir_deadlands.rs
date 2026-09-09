//! Ifnir Deadlands — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Pay 1 life: Add {B}.
//! Oracle: {2}{B}{B}, {T}, Sacrifice a Desert: Put two -1/-1 counters on target creature an opponent controls. Activate only as a sorcery.
//! Set: ECC #153 — Lorwyn Eclipsed Commander | Scryfall ID: 902e260d-71ba-4342-9794-fefe2b531c00 | Oracle ID: af698bd5-5f56-4d2a-9f02-8c3e781210cd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 648,
    oracle_id: "af698bd5-5f56-4d2a-9f02-8c3e781210cd",
    scryfall_id: "902e260d-71ba-4342-9794-fefe2b531c00",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Ifnir Deadlands",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
