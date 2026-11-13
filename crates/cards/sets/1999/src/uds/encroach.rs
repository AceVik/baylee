//! Encroach — {B} — Sorcery
//! Oracle: Target player reveals their hand. You choose a nonbasic land card from it. That player discards that card.
//! Set: UDS #59 — Urza's Destiny | Scryfall ID: fbd48dac-0a1a-49c4-8daf-11972b990454 | Oracle ID: b9959d63-5489-4ed1-a04e-9d6ba4a5f68e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static NONBASIC_LAND: Filter = Filter::And(&[Filter::LAND, Filter::Not(&Filter::BASIC_LAND)]);

card!(
    index = index::ENCROACH,
    oracle_id = "b9959d63-5489-4ed1-a04e-9d6ba4a5f68e",
    scryfall_id = "fbd48dac-0a1a-49c4-8daf-11972b990454",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Encroach",
        mana_cost = mana!("{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::RevealHandDiscard {
            filter: &NONBASIC_LAND,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
