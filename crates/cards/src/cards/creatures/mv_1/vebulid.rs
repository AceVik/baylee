//! Vebulid — {B} — Creature — Phyrexian Horror
//! Oracle: This creature enters with a +1/+1 counter on it.
//! Oracle: At the beginning of your upkeep, you may put a +1/+1 counter on this creature.
//! Oracle: When this creature attacks or blocks, destroy it at end of combat.
//! Set: USG #165 — Urza's Saga | Scryfall ID: d5623194-0cd9-4f3f-bab1-133d6b1e94fe | Oracle ID: a887def5-c9e1-4cbb-8b08-79b7bd808deb
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VEBULID,
    oracle_id = "a887def5-c9e1-4cbb-8b08-79b7bd808deb",
    scryfall_id = "d5623194-0cd9-4f3f-bab1-133d6b1e94fe",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Vebulid",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::HORROR],
        power = Some(0),
        toughness = Some(0),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::MayDo {
                effects: &[Effect::AddCounterFilter {
                    filter: &Filter::This,
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                }],
            }],
        ),
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::AtEndOfCombat {
                about: TargetSpec::EventObject,
                effects: &[Effect::destroy(TargetSpec::EventObject)],
            }],
        ),
        triggered!(
            Trigger::Blocks(&Filter::This),
            &[Effect::AtEndOfCombat {
                about: TargetSpec::EventObject,
                effects: &[Effect::destroy(TargetSpec::EventObject)],
            }],
        ),
    ],
);
