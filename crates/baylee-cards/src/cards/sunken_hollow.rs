//! Sunken Hollow — (no cost) — Land
//! Oracle: Sunken Hollow enters the battlefield tapped unless you control a SWAMP or an FOREST.
//! {T}: Add Black or Green.
//! Set: BFZ #249 — Battle for Zendikar | Scryfall ID: 3a8eef9b-9b03-42cd-a27a-07021bf0b33f | Oracle ID: cd2c90ac-2b04-461c-92f3-939871b6b6a3
// IMPLEMENTED — checkland (ETB tapped unless you control a SWAMP/FOREST) + 2-color mana.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, Coverage,
    Effect, EnterModifier, FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::land;
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static CHECK: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasType(TypeSet::LAND),
    Filter::Or(&[
        Filter::HasSubtype(land::SWAMP),
        Filter::HasSubtype(land::FOREST),
    ]),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(159),
    oracle_id: "cd2c90ac-2b04-461c-92f3-939871b6b6a3",
    scryfall_id: "3a8eef9b-9b03-42cd-a27a-07021bf0b33f",
    faces: &[FaceDef {
        name: "Sunken Hollow",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[EnterModifier::TappedUnless(&CHECK)],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddManaChoice {
            colors: &[ManaColor::Black, ManaColor::Green],
            amount: Amount::Fixed(1),
            combination: false,
        }],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
    }],
};

#[cfg(test)]
mod tests {}
