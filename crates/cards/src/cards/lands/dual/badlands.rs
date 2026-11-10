//! Badlands — (no cost) — Land — Swamp Mountain
//! Oracle: ({T}: Add {B} or {R}.)
//! Set: VMA #291 — Vintage Masters | Scryfall ID: 73403d04-fe97-4830-8b80-16dd1a1a6cc1 | Oracle ID: 13ff3222-91cb-4796-a34e-899ed817694c
// IMPLEMENTED — mana from current basic land types (CR 305.6).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SUBS: &[SubtypeId] = &[land::SWAMP, land::MOUNTAIN];

card!(
    index = index::BADLANDS,
    oracle_id = "13ff3222-91cb-4796-a34e-899ed817694c",
    scryfall_id = "73403d04-fe97-4830-8b80-16dd1a1a6cc1",
    faces = &[face!(
        name = "Badlands",
        types = TypeSet::LAND,
        subtypes = SUBS,
    )],
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::intrinsic_mana()])],
);
