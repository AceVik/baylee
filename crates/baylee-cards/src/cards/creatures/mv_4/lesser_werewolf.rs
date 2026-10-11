//! Lesser Werewolf — {3}{B} — Creature — Werewolf
//! Oracle: {B}: If this creature's power is 1 or more, it gets -1/-0 until end of turn and put a -0/-1 counter on target creature blocking or blocked by this creature. Activate only during the declare blockers step.
//! Set: ME3 #71 — Masters Edition III | Scryfall ID: dcbda60c-5a53-4232-b89c-267f7b11c192 | Oracle ID: 5542afeb-285f-4889-9d09-46ce20d0d189

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LESSER_WEREWOLF,
    oracle_id = "5542afeb-285f-4889-9d09-46ce20d0d189",
    scryfall_id = "dcbda60c-5a53-4232-b89c-267f7b11c192",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lesser Werewolf",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WEREWOLF],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        cost!("{B}"),
        &[Effect::IfCondition {
            condition: Condition::SourceMatches(&Filter::PowerAtLeast(1)),
            then: &[
                Effect::ModifySource {
                    modifier: Modifier::ModifyPT(-1, 0),
                    duration: Duration::UntilEndOfTurn
                },
                Effect::AddCounter {
                    kind: CounterKind::Minus {
                        power: 0,
                        toughness: 1
                    },
                    amount: Amount::Fixed(1)
                }
            ],
            otherwise: &[],
        }],
        target = Some(TargetSpec::Object(&Filter::And(&[
            Filter::CREATURE,
            Filter::Or(&[Filter::BlockingSource, Filter::BlockedBySource])
        ]))),
        condition = Some(Condition::DuringStep(StepKind::DeclareBlockers))
    ),],
);
