//! Tundra — (no cost) — Land — Plains Island
//! Oracle: ({T}: Add {W} or {U}.)
//! Set: VMA #322 — Vintage Masters | Scryfall ID: efd35cb4-862d-4699-a197-b744989b3ceb | Oracle ID: 02418479-9455-417f-a6a1-004356faff37
// IMPLEMENTED — mana from current basic land types (CR 305.6).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SUBS: &[SubtypeId] = &[land::PLAINS, land::ISLAND];

card!(
    index = index::TUNDRA,
    oracle_id = "02418479-9455-417f-a6a1-004356faff37",
    scryfall_id = "efd35cb4-862d-4699-a197-b744989b3ceb",
    faces = &[face!(
        name = "Tundra",
        types = TypeSet::LAND,
        subtypes = SUBS,
    )],
    color_identity = ColorSet::from_slice(&[Color::White, Color::Blue]),
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::intrinsic_mana()])],
);
