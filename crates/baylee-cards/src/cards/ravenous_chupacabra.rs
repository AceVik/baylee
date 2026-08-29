//! Ravenous Chupacabra — {2}{B}{B} — Creature — Beast Horror
//! Oracle: When this creature enters, destroy target creature an opponent controls.
//! Set: MKC #136 — Murders at Karlov Manor Commander | Scryfall ID: a4dfbac0-1849-41c5-853a-1fee108d0b01 | Oracle ID: 7b459306-149b-4f43-abc1-2dd70c748c0e
// IMPLEMENTED — ETB kill on an opponent's creature.
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
    index: CardIndex::new(124),
    oracle_id: "7b459306-149b-4f43-abc1-2dd70c748c0e",
    scryfall_id: "a4dfbac0-1849-41c5-853a-1fee108d0b01",
    faces: &[FaceDef {
        name: "Ravenous Chupacabra",
        mana_cost: baylee_core::mana!("{2}{B}{B}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::BEAST, creature::HORROR],
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
        cost_reduction: None,
        disturb: false,
        adventure: false,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        once_per_turn: false,
        effects: &[Effect::Destroy {
            target: TargetSpec::Object(&ENEMY_CREATURE),
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&ENEMY_CREATURE))),
    }],
};

#[cfg(test)]
mod tests {}
