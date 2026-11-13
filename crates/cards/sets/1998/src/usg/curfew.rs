//! Curfew — {U} — Instant
//! Oracle: Each player returns a creature they control to its owner's hand.
//! Set: USG #68 — Urza's Saga | Scryfall ID: 49ee9af3-d61c-4964-88a6-6e8ad6a6a29a | Oracle ID: 6ff1e793-1569-4887-b6bb-d9855c5e8a26
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CURFEW,
    oracle_id = "6ff1e793-1569-4887-b6bb-d9855c5e8a26",
    scryfall_id = "49ee9af3-d61c-4964-88a6-6e8ad6a6a29a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Curfew",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::ReturnChosenToHand {
        who: PlayerRel::EachPlayer,
        filter: &Filter::CREATURE,
    }])],
);
