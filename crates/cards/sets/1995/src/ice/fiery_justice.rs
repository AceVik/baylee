//! Fiery Justice — {R}{G}{W} — Sorcery
//! Oracle: Fiery Justice deals 5 damage divided as you choose among any number of targets. Target opponent gains 5 life.
//! Set: 2X2 #212 — Double Masters 2022 | Scryfall ID: 144668d6-cab6-45e6-8498-ab7cd927f9df | Oracle ID: 333809cb-e196-45f2-8a67-31374438e56e

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIERY_JUSTICE,
    oracle_id = "333809cb-e196-45f2-8a67-31374438e56e",
    scryfall_id = "144668d6-cab6-45e6-8498-ab7cd927f9df",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fiery Justice",
        mana_cost = mana!("{R}{G}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::DealDamageDivided {
                amount: DivisionAmount::Fixed(5),
            },
            Effect::GainLifeFor {
                amount: Amount::Fixed(5),
                who: PlayerRel::SecondTargetPlayerOrController,
            },
        ],
        targets = Some(TargetReq::up_to(TargetSpec::AnyTarget, 5)),
        second_targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
