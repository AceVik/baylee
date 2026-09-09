//! Underground Sea — (no cost) — Land — ISLAND SWAMP
//! Oracle: ({T}: Add {B} or {B}.)
//! Set: VMA #323 — Vintage Masters | Scryfall ID: 26cee543-6eab-494e-a803-33a5d48d7d74 | Oracle ID: 4b22be3a-8ce1-47d1-b82e-6c3ccfb0548b
// IMPLEMENTED — two-color mana choice.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static COLORS: &[ManaColor] = &[ManaColor::Blue, ManaColor::Black];
static SUBS: &[SubtypeId] = &[land::ISLAND, land::SWAMP];

card! {
    index: 178,
    oracle_id: "4b22be3a-8ce1-47d1-b82e-6c3ccfb0548b",
    scryfall_id: "26cee543-6eab-494e-a803-33a5d48d7d74",
    faces: &[face! {
        name: "Underground Sea",
        types: TypeSet::LAND,
        subtypes: SUBS,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_choice(COLORS)])],
}
