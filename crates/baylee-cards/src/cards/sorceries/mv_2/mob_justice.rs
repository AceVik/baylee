//! Mob Justice — {1}{R} — Sorcery
//! Oracle: Mob Justice deals damage to target player or planeswalker equal to the number of creatures you control.
//! Set: STH #90 — Stronghold | Scryfall ID: b790d789-bb21-4119-a0e3-43af9bef8acc | Oracle ID: 0ff97869-f27c-4379-92fd-605062aa5418
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOB_JUSTICE,
    oracle_id = "0ff97869-f27c-4379-92fd-605062aa5418",
    scryfall_id = "b790d789-bb21-4119-a0e3-43af9bef8acc",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mob Justice",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamage {
            target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER),
            amount: Amount::CountOf {
                zone: ZoneSel::Battlefield,
                filter: &Filter::YOUR_CREATURE,
            },
        }],
        targets = Some(TargetReq::one(TargetSpec::PlayerOrObject(
            &Filter::PLANESWALKER
        ))),
    )],
);
