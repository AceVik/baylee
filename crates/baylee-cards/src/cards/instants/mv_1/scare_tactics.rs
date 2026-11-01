//! Scare Tactics — {B} — Instant
//! Oracle: Creatures you control get +1/+0 until end of turn.
//! Set: EXO #73 — Exodus | Scryfall ID: 6a9d4e11-ce2e-445a-9536-756a6687d6d7 | Oracle ID: d7548770-5c60-4c3f-b1d6-554ab5f931f9
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCARE_TACTICS,
    oracle_id = "d7548770-5c60-4c3f-b1d6-554ab5f931f9",
    scryfall_id = "6a9d4e11-ce2e-445a-9536-756a6687d6d7",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Scare Tactics",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::PumpFilter {
        filter: &Filter::YOUR_CREATURE,
        controlled_by: None,
        power: Amount::Fixed(1),
        toughness: Amount::Fixed(0),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn
    }]),],
);
