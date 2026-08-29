//! Jin-Gitaxias, Progress Tyrant — {5}{U}{U} — Legendary Creature — Phyrexian Praetor
//! Oracle: Whenever you cast an artifact, instant, or sorcery spell, copy that spell. You may choose new targets for the copy. This ability triggers only once each turn. (A copy of a permanent spell becomes a token.)
//! Oracle: Whenever an opponent casts an artifact, instant, or sorcery spell, counter that spell. This ability triggers only once each turn.
//! Set: NEO #59 — Kamigawa: Neon Dynasty | Scryfall ID: c57b4876-5387-4f73-b8e2-8e7bdca8b0bc | Oracle ID: f5daadc1-98ff-480a-82bb-fe7bfaa7b60e
// IMPLEMENTED — once-per-turn spell copy + once-per-turn counter.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static YOUR_AIS_SPELL: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::Or(&[
        Filter::HasType(TypeSet::ARTIFACT),
        Filter::HasType(TypeSet::INSTANT),
        Filter::HasType(TypeSet::SORCERY),
    ]),
]);
static OPPONENT_AIS_SPELL: Filter = Filter::And(&[
    Filter::ControlledByOpponent,
    Filter::Or(&[
        Filter::HasType(TypeSet::ARTIFACT),
        Filter::HasType(TypeSet::INSTANT),
        Filter::HasType(TypeSet::SORCERY),
    ]),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(78),
    oracle_id: "f5daadc1-98ff-480a-82bb-fe7bfaa7b60e",
    scryfall_id: "c57b4876-5387-4f73-b8e2-8e7bdca8b0bc",
    faces: &[FaceDef {
        name: "Jin-Gitaxias, Progress Tyrant",
        mana_cost: baylee_core::mana!("{5}{U}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[creature::PHYREXIAN, creature::PRAETOR],
        power: Some(5),
        toughness: Some(5),
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
        cost_reduction: None,
        disturb: false,
        adventure: false,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("target re-choice for the copy (protocol M3)"),
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::SpellCast(&YOUR_AIS_SPELL),
            once_per_turn: true,
            effects: &[Effect::CopyTargetSpell { mods: &[] }],
            targets: Some(baylee_cards_dsl::TargetReq::one(
                baylee_cards_dsl::TargetSpec::EventObject,
            )),
        },
        AbilityDef::Triggered {
            trigger: Trigger::SpellCast(&OPPONENT_AIS_SPELL),
            once_per_turn: true,
            effects: &[Effect::CounterTargetSpell],
            targets: Some(baylee_cards_dsl::TargetReq::one(
                baylee_cards_dsl::TargetSpec::EventObject,
            )),
        },
    ],
};

#[cfg(test)]
mod tests {}
