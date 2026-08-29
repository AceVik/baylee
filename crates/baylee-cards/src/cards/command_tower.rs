//! Command Tower — (no cost) — Land
//! Oracle: {T}: Add one mana of any color in your commander's color identity.
//! Set: MSC #233 — Marvel Super Heroes Commander | Scryfall ID: 0548fb60-c843-4f8f-a029-6f10efc63a41 | Oracle ID: 0895c9b7-ae7d-4bb3-af17-3b75deb50a25
// IMPLEMENTED — color choice from the union of your command-zone cards'
// color identities at resolution; falls back to {C} without a commander.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Effect,
    FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(23),
    oracle_id: "0895c9b7-ae7d-4bb3-af17-3b75deb50a25",
    scryfall_id: "0548fb60-c843-4f8f-a029-6f10efc63a41",
    faces: &[FaceDef {
        name: "Command Tower",
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
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
        delve: false,
        convoke: false,
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddManaCommanderIdentity],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
    }],
};

#[cfg(test)]
mod tests {}
