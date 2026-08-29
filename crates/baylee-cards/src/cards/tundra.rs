//! Tundra — (no cost) — Land — PLAINS ISLAND
//! Oracle: ({T}: Add {W} or {B}.)
//! Set: VMA #322 — Vintage Masters | Scryfall ID: efd35cb4-862d-4699-a197-b744989b3ceb | Oracle ID: 02418479-9455-417f-a6a1-004356faff37
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

static COLORS: &[ManaColor] = &[ManaColor::White, ManaColor::Blue];
static SUBS: &[baylee_core::ids::SubtypeId] = &[land::PLAINS, land::ISLAND];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(175),
    oracle_id: "02418479-9455-417f-a6a1-004356faff37",
    scryfall_id: "efd35cb4-862d-4699-a197-b744989b3ceb",
    faces: &[FaceDef {
        name: "Tundra",
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
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue]),
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
