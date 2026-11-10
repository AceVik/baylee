//! Sleeper Agent — {B} — Creature — Phyrexian Minion
//! Oracle: When this creature enters, target opponent gains control of it.
//! Oracle: At the beginning of your upkeep, this creature deals 2 damage to you.
//! Set: USG #159 — Urza's Saga | Scryfall ID: b1b2c4bd-2397-4ebd-b4fb-3b3e9c6c7dec | Oracle ID: 627c9546-85b0-4b4d-93c6-e2ca8c3adf4f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SLEEPER_AGENT,
    oracle_id = "627c9546-85b0-4b4d-93c6-e2ca8c3adf4f",
    scryfall_id = "b1b2c4bd-2397-4ebd-b4fb-3b3e9c6c7dec",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sleeper Agent",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::MINION],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::GainControlAll {
                filter: &Filter::This,
                to: PlayerRel::TargetPlayerOrController,
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::Player(PlayerRel::You),
            }],
        ),
    ],
);
