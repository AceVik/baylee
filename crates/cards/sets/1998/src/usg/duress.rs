//! Duress — {B} — Sorcery
//! Oracle: Target opponent reveals their hand. You choose a noncreature, nonland card from it. That player discards that card.
//! Set: USG #132 — Urza's Saga | Scryfall ID: ca367f49-0f4a-4b7f-8104-851893fbcd8a | Oracle ID: 33d405ea-7a9a-4970-b70f-9c05d90dd6f0
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DURESS,
    oracle_id = "33d405ea-7a9a-4970-b70f-9c05d90dd6f0",
    scryfall_id = "ca367f49-0f4a-4b7f-8104-851893fbcd8a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Duress",
        mana_cost = mana!("{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::RevealHandDiscard {
            filter: &Filter::And(&[Filter::Not(&Filter::CREATURE), Filter::Not(&Filter::LAND)]),
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
