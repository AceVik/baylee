//! Kindle — {1}{R} — Instant
//! Oracle: Kindle deals X damage to any target, where X is 2 plus the number of cards named Kindle in all graveyards.
//! Set: A25 #140 — Masters 25 | Scryfall ID: f687518f-73ff-4813-a56f-5b5cc4f36e7a | Oracle ID: 9397a23b-770a-43fc-8bc2-2c3abb0683c5
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const KINDLE_COUNT: Amount = Amount::CountOf {
    filter: &Filter::Named("Kindle"),
    zone: ZoneSel::GraveyardAll,
};

card!(
    index = index::KINDLE,
    oracle_id = "9397a23b-770a-43fc-8bc2-2c3abb0683c5",
    scryfall_id = "f687518f-73ff-4813-a56f-5b5cc4f36e7a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Kindle",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamage {
            target: TargetSpec::AnyTarget,
            amount: Amount::Plus {
                base: &KINDLE_COUNT,
                offset: 2,
            },
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
