//! Savannah — (no cost) — Land — FOREST PLAINS
//! Oracle: ({T}: Add {G} or {W}.)
//! Set: VMA #311 — Vintage Masters | Scryfall ID: b0d161fc-4a2a-4f1d-82b4-a746552552df | Oracle ID: 703243f0-8cb3-420f-958f-5fd4bde30293
// IMPLEMENTED — two-color mana choice.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, land};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static COLORS: &[ManaColor] = &[ManaColor::Green, ManaColor::White];
static SUBS: &[baylee_core::ids::SubtypeId] = &[land::FOREST, land::PLAINS];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(138),
    oracle_id: "703243f0-8cb3-420f-958f-5fd4bde30293",
    scryfall_id: "b0d161fc-4a2a-4f1d-82b4-a746552552df",
    faces: &[FaceDef {
        name: "Savannah",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: SUBS,
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
    color_identity: ColorSet::from_slice(&[Color::Green, Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddManaChoice {
            colors: COLORS,
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
