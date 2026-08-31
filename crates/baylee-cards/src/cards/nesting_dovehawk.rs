//! Nesting Dovehawk — {2}{W} — Creature — Bird
//! Oracle: Flying
//! Oracle: At the beginning of combat on your turn, populate. (Create a token that's a copy of a creature token you control.)
//! Oracle: Whenever a creature token you control enters, put a +1/+1 counter on this creature.
//! Set: EOC #25 — Edge of Eternities Commander | Scryfall ID: c58ff93f-7135-40af-92ce-358da48694dc | Oracle ID: fe8fc442-ed17-40b2-8624-69f2eed3f9be
// IMPLEMENTED — populate (token-only copy) + token-ETB growth.
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

static CREATURE_TOKEN_YOU_CONTROL: Filter = Filter::And(&[
    Filter::IsToken,
    Filter::HasType(TypeSet::CREATURE),
    Filter::ControlledByYou,
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(103),
    oracle_id: "fe8fc442-ed17-40b2-8624-69f2eed3f9be",
    scryfall_id: "c58ff93f-7135-40af-92ce-358da48694dc",
    faces: &[FaceDef {
        name: "Nesting Dovehawk",
        mana_cost: baylee_core::mana!("{2}{W}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::BIRD],
        power: Some(2),
        toughness: Some(2),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::FLYING,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::StepBegin {
                step: StepKind::CombatBegin,
                whose: baylee_cards_dsl::PlayerRel::You,
            },
            once_per_turn: false,
            effects: &[Effect::CreateTokenCopyOf {
                target: Some(TargetSpec::Object(&CREATURE_TOKEN_YOU_CONTROL)),
                kicked_bonus: 0,
            }],
            targets: Some(TargetReq {
                spec: TargetSpec::Object(&CREATURE_TOKEN_YOU_CONTROL),
                min: 0,
                max: 1,
                count_is_x: false,
            }),
        },
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefield(&CREATURE_TOKEN_YOU_CONTROL),
            once_per_turn: false,
            effects: &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
            targets: None,
        },
    ],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
