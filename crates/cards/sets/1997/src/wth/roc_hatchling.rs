//! Roc Hatchling — {R} — Creature — Bird
//! Oracle: This creature enters with four shell counters on it.
//! Oracle: At the beginning of your upkeep, remove a shell counter from this creature.
//! Oracle: As long as this creature has no shell counters on it, it gets +3/+2 and has flying.
//! Set: WTH #113 — Weatherlight | Scryfall ID: 25857884-6bb7-4a8e-a08b-fa610af8a5c3 | Oracle ID: 6764d4d9-1fe5-4b42-830c-a3b4c3b37f1f
// IMPLEMENTED — Enters with four shell counters, removes one at your upkeep, and gets +3/+2 and flying while it has no shell counters.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ROC_HATCHLING,
    oracle_id = "6764d4d9-1fe5-4b42-830c-a3b4c3b37f1f",
    scryfall_id = "25857884-6bb7-4a8e-a08b-fa610af8a5c3",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Roc Hatchling",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD],
        power = Some(0),
        toughness = Some(1),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: counters::SHELL,
            amount: Amount::Fixed(4),
        }],
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::RemoveCounterSelf {
                kind: counters::SHELL,
                n: 1,
            }],
        ),
        static_ability!(
            Filter::This,
            Modifier::ModifyPT(3, 2),
            condition = Some(Condition::CountersOnSelfExactly(counters::SHELL, 0)),
        ),
        static_ability!(
            Filter::This,
            Modifier::AddKeyword(KeywordSet::FLYING),
            condition = Some(Condition::CountersOnSelfExactly(counters::SHELL, 0)),
        ),
    ],
);
