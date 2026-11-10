//! River of Tears — (no cost) — Land
//! Oracle: {T}: Add {U}. If you played a land this turn, add {B} instead.
//! Set: MKC #283 — Murders at Karlov Manor Commander | Scryfall ID: 67b626cc-1c12-4059-afa5-e5a1221ea1ba | Oracle ID: 8a83d284-75a0-4901-b7d9-c4b7586ee327

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RIVER_OF_TEARS,
    oracle_id = "8a83d284-75a0-4901-b7d9-c4b7586ee327",
    scryfall_id = "67b626cc-1c12-4059-afa5-e5a1221ea1ba",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(name = "River of Tears", types = TypeSet::LAND,)],
    abilities = &[mana_ability!(&[Effect::IfCondition {
        condition: Condition::LandsPlayedThisTurnAtLeast(1),
        then: &[Effect::mana(ManaColor::Black, 1)],
        otherwise: &[Effect::mana(ManaColor::Blue, 1)],
    }])],
);
