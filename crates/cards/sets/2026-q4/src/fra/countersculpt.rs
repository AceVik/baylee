//! Countersculpt — {U}{U} — Instant
//! Oracle: As an additional cost to cast this spell, behold a Jace or pay {1}. (To behold a Jace, choose a Jace you control or reveal a Jace card from your hand.)
//! Oracle: Counter target spell. Empower Jace 1. (Put a loyalty counter on a Jace token you control. If you don't control one, first create a blue Jace planeswalker token with "[−1]: Surveil 1" and "[−3]: Draw a card.")
//! Set: FRA #25 — Reality Fracture | Scryfall ID: 145b928d-a7ff-4fe5-ae4d-bbae7b1d955b | Oracle ID: fc3353e9-fe24-4ca7-bff8-9da767f2903a
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static JACE: Filter = Filter::HasSubtype(subtypes::planeswalker::JACE);

card!(
    index = index::COUNTERSCULPT,
    oracle_id = "fc3353e9-fe24-4ca7-bff8-9da767f2903a",
    scryfall_id = "145b928d-a7ff-4fe5-ae4d-bbae7b1d955b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Countersculpt",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::INSTANT,
        behold_or_pay = Some(BeholdOrPay {
            filter: &JACE,
            cost: mana!("{1}"),
        }),
    ),],
    abilities = &[spell!(
        &[
            Effect::CounterTargetSpell,
            Effect::EmpowerJace {
                token: &crate::tokens::JACE_PLANESWALKER_BLUE,
                amount: 1,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )],
);
