//! City of Brass — (no cost) — Land
//! Oracle: Whenever this land becomes tapped, it deals 1 damage to you.
//! Oracle: {T}: Add one mana of any color.
//! Set: TMC #62 — Teenage Mutant Ninja Turtles Eternal | Scryfall ID: c21565d0-fc40-4d89-9b27-87c03385e0af | Oracle ID: f25351e3-539b-4bbc-b92d-6480acf4d722
// PARTIAL — any-color mana implemented; the becomes-tapped damage
// trigger needs a tap event/trigger kind (own milestone).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_COLOR: &[ManaColor] = &[
    ManaColor::White,
    ManaColor::Blue,
    ManaColor::Black,
    ManaColor::Red,
    ManaColor::Green,
];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(20),
    oracle_id: "f25351e3-539b-4bbc-b92d-6480acf4d722",
    scryfall_id: "c21565d0-fc40-4d89-9b27-87c03385e0af",
    faces: &[FaceDef {
        name: "City of Brass",
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
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("becomes-tapped damage trigger (tap event kind, own milestone)"),
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddManaChoice {
            colors: ANY_COLOR,
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
