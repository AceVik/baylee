//! Homarid — {2}{U} — Creature — Homarid
//! Oracle: This creature enters with a tide counter on it.
//! Oracle: At the beginning of your upkeep, put a tide counter on this creature.
//! Oracle: As long as there is exactly one tide counter on this creature, it gets -1/-1.
//! Oracle: As long as there are exactly three tide counters on this creature, it gets +1/+1.
//! Oracle: Whenever there are four or more tide counters on this creature, remove all tide counters from it.
//! Set: FEM #19a — Fallen Empires | Scryfall ID: d6ffeab4-83b1-4414-ae72-e59a2354ea15 | Oracle ID: 5d02b0d9-cc53-4967-84bb-8b7f5c489b5c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const FOUR_OR_MORE_TIDE: Condition = Condition::CountersOnSelf(counters::TIDE, 4);

card!(
    index = index::HOMARID,
    oracle_id = "5d02b0d9-cc53-4967-84bb-8b7f5c489b5c",
    scryfall_id = "d6ffeab4-83b1-4414-ae72-e59a2354ea15",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Homarid",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HOMARID],
        power = Some(2),
        toughness = Some(2),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: counters::TIDE,
            amount: Amount::Fixed(1),
        }],
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::AddCounter {
                kind: counters::TIDE,
                amount: Amount::Fixed(1),
            }],
        ),
        static_ability!(
            Filter::This,
            Modifier::ModifyPT(-1, -1),
            condition = Some(Condition::CountersOnSelfExactly(counters::TIDE, 1)),
        ),
        static_ability!(
            Filter::This,
            Modifier::ModifyPT(1, 1),
            condition = Some(Condition::CountersOnSelfExactly(counters::TIDE, 3)),
        ),
        triggered!(
            Trigger::State(&FOUR_OR_MORE_TIDE),
            &[Effect::RemoveAllCountersSelf {
                kind: counters::TIDE,
            }],
        ),
    ],
);
