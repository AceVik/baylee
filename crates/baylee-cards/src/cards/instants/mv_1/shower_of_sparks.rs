//! Shower of Sparks — {R} — Instant
//! Oracle: Shower of Sparks deals 1 damage to target creature and 1 damage to target player or planeswalker.
//! Set: USG #217 — Urza's Saga | Scryfall ID: 54428999-a83d-40a5-9753-dfefdf705a9e | Oracle ID: a43500ed-4aa6-49de-8a6e-a1781301b108
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHOWER_OF_SPARKS,
    oracle_id = "a43500ed-4aa6-49de-8a6e-a1781301b108",
    scryfall_id = "54428999-a83d-40a5-9753-dfefdf705a9e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shower of Sparks",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamageToBothTargetInstances {
            amount: Amount::Fixed(1),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets = Some(TargetReq::one(TargetSpec::PlayerOrObject(
            &Filter::PLANESWALKER,
        ))),
    ),],
);
