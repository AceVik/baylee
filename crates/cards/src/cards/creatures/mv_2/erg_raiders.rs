//! Erg Raiders — {1}{B} — Creature — Human Warrior
//! Oracle: At the beginning of your end step, if this creature didn't attack this turn, it deals 2 damage to you unless it came under your control this turn.
//! Set: A25 #90 — Masters 25 | Scryfall ID: b29d88ad-5074-4f37-bb6a-2a7fe75c5771 | Oracle ID: 7feba745-7d27-4225-bc9d-9b7a8692872d
// IMPLEMENTED — end-step trigger gated on not attacking; the 2 damage is skipped when it came under your control this turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ERG_RAIDERS,
    oracle_id = "7feba745-7d27-4225-bc9d-9b7a8692872d",
    scryfall_id = "b29d88ad-5074-4f37-bb6a-2a7fe75c5771",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Erg Raiders",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARRIOR],
        power = Some(2),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::End,
            whose: PlayerRel::You
        },
        &[Effect::IfCondition {
            condition: Condition::SourceMatches(&Filter::ControlledSinceTurnBegan),
            then: &[Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::Player(PlayerRel::You)
            }],
            otherwise: &[],
        }],
        condition = Some(Condition::SourceMatches(&Filter::Not(
            &Filter::AttackedThisTurn
        )))
    )],
);
