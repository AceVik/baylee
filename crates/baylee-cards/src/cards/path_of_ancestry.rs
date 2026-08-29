//! Path of Ancestry — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add one mana of any color in your commander's color identity. When that mana is spent to cast a creature spell that shares a creature type with your commander, scry 1. (Look at the top card of your library. You may put that card on the bottom.)
//! Set: MBC #80 — Mystery Booster Commander Edition | Scryfall ID: b1aaa7b0-1cac-4a92-b880-7ef1ac00618f | Oracle ID: b473e293-59e3-4e04-acf2-622604aeb25f
// PARTIAL — enters tapped + commander-identity mana implemented; the
// scry rider needs mana-source tracking (own milestone, same class as
// Cavern of Souls).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Effect,
    EnterModifier, FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(111),
    oracle_id: "b473e293-59e3-4e04-acf2-622604aeb25f",
    scryfall_id: "b1aaa7b0-1cac-4a92-b880-7ef1ac00618f",
    faces: &[FaceDef {
        name: "Path of Ancestry",
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
        enter_modifiers: &[EnterModifier::Tapped],
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
    coverage: Coverage::Partial("scry-on-spend rider (mana-source tracking, own milestone)"),
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
