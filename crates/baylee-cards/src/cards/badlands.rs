//! Badlands — (no cost) — Land — SWAMP MOUNTAIN
//! Oracle: ({T}: Add {B} or {R}.)
//! Set: VMA #291 — Vintage Masters | Scryfall ID: 73403d04-fe97-4830-8b80-16dd1a1a6cc1 | Oracle ID: 13ff3222-91cb-4796-a34e-899ed817694c
// IMPLEMENTED — two-color mana choice.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static COLORS: &[ManaColor] = &[ManaColor::Black, ManaColor::Red];
static SUBS: &[SubtypeId] = &[land::SWAMP, land::MOUNTAIN];

card! {
    index: 9,
    oracle_id: "13ff3222-91cb-4796-a34e-899ed817694c",
    scryfall_id: "73403d04-fe97-4830-8b80-16dd1a1a6cc1",
    faces: &[face! {
        name: "Badlands",
        types: TypeSet::LAND,
        subtypes: SUBS,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_choice(COLORS)])],
}
