//! Insubordination — {B}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: At the beginning of the end step of enchanted creature's controller, this Aura deals 2 damage to that player unless that creature attacked this turn.
//! Set: MMQ #141 — Mercadian Masques | Scryfall ID: d2544c9d-adc2-4d67-8850-9af38e73ea1e | Oracle ID: 0d3e843e-eeb4-421e-b55d-ceb21f51a080
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const DID_NOT_ATTACK_THIS_TURN: Condition =
    Condition::Not(&Condition::AttachedMatches(&Filter::AttackedThisTurn));

card!(
    index = index::INSUBORDINATION,
    oracle_id = "0d3e843e-eeb4-421e-b55d-ceb21f51a080",
    scryfall_id = "d2544c9d-adc2-4d67-8850-9af38e73ea1e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Insubordination",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::ControllerOfAttached,
            },
            &[Effect::IfCondition {
                condition: DID_NOT_ATTACK_THIS_TURN,
                then: &[Effect::DealDamage {
                    amount: Amount::Fixed(2),
                    target: TargetSpec::Player(PlayerRel::EventPlayer),
                }],
                otherwise: &[],
            }],
        ),
    ],
);
