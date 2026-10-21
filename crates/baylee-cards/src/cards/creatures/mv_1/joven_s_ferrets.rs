//! Joven's Ferrets — {G} — Creature — Ferret
//! Oracle: Whenever this creature attacks, it gets +0/+2 until end of turn.
//! Oracle: At end of combat, tap all creatures that blocked this creature this turn. They don't untap during their controller's next untap step.
//! Set: ME2 #169 — Masters Edition II | Scryfall ID: 3765ab78-a645-4e4f-9f83-5a6bc762b8d0 | Oracle ID: d2817b5f-f7a2-4254-a00f-e2e45d7a8cc0
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const BLOCKED_SOURCE: Filter = Filter::And(&[Filter::CREATURE, Filter::BlockingSource]);

card!(
    index = index::JOVEN_S_FERRETS,
    oracle_id = "d2817b5f-f7a2-4254-a00f-e2e45d7a8cc0",
    scryfall_id = "3765ab78-a645-4e4f-9f83-5a6bc762b8d0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Joven's Ferrets",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FERRET],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::ModifySource {
                modifier: Modifier::ModifyPT(0, 2),
                duration: Duration::UntilEndOfTurn,
            }],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::CombatEnd,
                whose: PlayerRel::EachPlayer,
            },
            &[
                Effect::TapAll {
                    filter: &BLOCKED_SOURCE,
                },
                Effect::ForEachMatching {
                    filter: &BLOCKED_SOURCE,
                    effects: &[Effect::SuppressUntap {
                        target: TargetSpec::EventObject,
                        steps: 1,
                    }],
                },
            ],
        ),
    ],
);
