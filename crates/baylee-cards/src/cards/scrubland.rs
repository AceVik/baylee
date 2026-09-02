//! Scrubland — (no cost) — Land — PLAINS SWAMP
//! Oracle: ({T}: Add {W} or {B}.)
//! Set: VMA #313 — Vintage Masters | Scryfall ID: 9d471e36-a3ab-4a96-ba4b-8eca921ea37a | Oracle ID: c8d95ca8-7d12-4072-aeaf-e20f248c7e39
// IMPLEMENTED — two-color mana choice.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static COLORS: &[ManaColor] = &[ManaColor::White, ManaColor::Black];
static SUBS: &[SubtypeId] = &[land::PLAINS, land::SWAMP];

card! {
    index: 140,
    oracle_id: "c8d95ca8-7d12-4072-aeaf-e20f248c7e39",
    scryfall_id: "9d471e36-a3ab-4a96-ba4b-8eca921ea37a",
    faces: &[face! {
        name: "Scrubland",
        types: TypeSet::LAND,
        subtypes: SUBS,
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_choice(COLORS)])],
}
