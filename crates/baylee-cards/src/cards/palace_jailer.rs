//! Palace Jailer — {2}{W}{W} — Creature — Human Soldier
//! Oracle: When this creature enters, you become the monarch.
//! Oracle: When this creature enters, exile target creature an opponent controls until an opponent becomes the monarch.
//! Set: C17 #21 — Commander 2017 | Scryfall ID: 3a8c2a84-e0f2-4611-af3d-42f4578ad4e3 | Oracle ID: 180eda7c-fca2-403b-85cd-8ffebaf9f408
// IMPLEMENTED — monarch designation (become monarch, monarch draw at end
// step, monarch-linked exile release).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ENEMY_CREATURE: Filter = Filter::And(&[
    Filter::ControlledByOpponent,
    Filter::HasType(TypeSet::CREATURE),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(109),
    oracle_id: "180eda7c-fca2-403b-85cd-8ffebaf9f408",
    scryfall_id: "3a8c2a84-e0f2-4611-af3d-42f4578ad4e3",
    faces: &[FaceDef {
        name: "Palace Jailer",
        mana_cost: baylee_core::mana!("{2}{W}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::HUMAN, creature::SOLDIER],
        power: Some(2),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
        delve: false,
        convoke: false,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefield(&Filter::This),
            once_per_turn: false,
            effects: &[Effect::BecomeMonarch],
            targets: None,
        },
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefield(&Filter::This),
            once_per_turn: false,
            effects: &[Effect::ExileLinked {
                target: TargetSpec::Object(&ENEMY_CREATURE),
            }],
            targets: Some(TargetReq::one(TargetSpec::Object(&ENEMY_CREATURE))),
        },
    ],
};

#[cfg(test)]
mod tests {}
