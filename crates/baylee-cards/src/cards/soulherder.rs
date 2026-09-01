//! Soulherder — {1}{W}{U} — Creature — Spirit
//! Oracle: Whenever a creature is exiled from the battlefield, put a +1/+1 counter on this creature.
//! Oracle: At the beginning of your end step, you may exile another target creature you control, then return that card to the battlefield under its owner's control.
//! Set: KHC #93 — Kaldheim Commander | Scryfall ID: 50bc0f5b-7421-45b9-af85-86dd9821b7d8 | Oracle ID: 92019547-f6db-4ea6-8356-d0a90ace5662
// IMPLEMENTED — exile-watcher growth + end-step blink.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, CounterKind, Coverage, Effect, FaceDef, Filter,
    KeywordSet, PartnerKind, StepKind, TargetReq, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_CREATURE: Filter = Filter::HasType(TypeSet::CREATURE);
static ANOTHER_CREATURE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::Another,
    Filter::HasType(TypeSet::CREATURE),
    Filter::ControlledByYou,
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(153),
    oracle_id: "92019547-f6db-4ea6-8356-d0a90ace5662",
    scryfall_id: "50bc0f5b-7421-45b9-af85-86dd9821b7d8",
    faces: &[FaceDef {
        name: "Soulherder",
        mana_cost: baylee_core::mana!("{1}{W}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::SPIRIT],
        power: Some(1),
        toughness: Some(1),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::ExiledFromBattlefield(&ANY_CREATURE),
            once_per_turn: false,
            effects: &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
            targets: None,
        },
        AbilityDef::Triggered {
            trigger: Trigger::StepBegin {
                step: StepKind::End,
                whose: baylee_cards_dsl::PlayerRel::You,
            },
            once_per_turn: false,
            effects: &[Effect::Blink {
                target: TargetSpec::Object(&ANOTHER_CREATURE_YOU_CONTROL),
            }],
            targets: Some(TargetReq {
                spec: TargetSpec::Object(&ANOTHER_CREATURE_YOU_CONTROL),
                min: 0,
                max: 1,
                count_is_x: false,
            }),
        },
    ],
    ..CardDef::DEFAULT
};
