//! Bulwark — {3}{R}{R} — Enchantment
//! Oracle: At the beginning of your upkeep, this enchantment deals X damage to target opponent, where X is the number of cards in your hand minus the number of cards in that player's hand.
//! Set: USG #178 — Urza's Saga | Scryfall ID: a2041b95-ec5b-4512-b012-f875ca686669 | Oracle ID: 259da825-02f7-4e72-8f44-01683330f457
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BULWARK,
    oracle_id = "259da825-02f7-4e72-8f44-01683330f457",
    scryfall_id = "a2041b95-ec5b-4512-b012-f875ca686669",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Bulwark",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::DealDamage {
            amount: Amount::Difference {
                left: &Amount::HandSize(PlayerRel::You),
                right: &Amount::HandSize(PlayerRel::TargetPlayerOrController),
            },
            target: TargetSpec::AnyOpponent,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
