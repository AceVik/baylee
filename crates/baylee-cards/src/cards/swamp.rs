//! Swamp — (no cost) — Basic Land — Swamp
//! Oracle: ({T}: Add {B}.)
//! Set: TRK #321 — Star Trek | Scryfall ID: b7387103-1df1-4fd0-9e91-1544509792c7 | Oracle ID: 56719f6a-1a6c-4c0a-8d21-18f7d7350b68
// IMPLEMENTED — basic land mana ability.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Effect,
    FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, land};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(162),
    oracle_id: "56719f6a-1a6c-4c0a-8d21-18f7d7350b68",
    scryfall_id: "b7387103-1df1-4fd0-9e91-1544509792c7",
    faces: &[FaceDef {
        name: "Swamp",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::BASIC,
        subtypes: &[land::SWAMP],
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
        cost_reduction: None,
        disturb: false,
        adventure: false,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddMana {
            color: ManaColor::Black,
            amount: 1,
        }],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
    }],
};

#[cfg(test)]
mod tests {}
