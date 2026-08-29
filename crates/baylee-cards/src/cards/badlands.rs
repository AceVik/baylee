//! Badlands — (no cost) — Land — SWAMP MOUNTAIN
//! Oracle: ({T}: Add {B} or {R}.)
//! Set: VMA #291 — Vintage Masters | Scryfall ID: 73403d04-fe97-4830-8b80-16dd1a1a6cc1 | Oracle ID: 13ff3222-91cb-4796-a34e-899ed817694c
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

static COLORS: &[ManaColor] = &[ManaColor::Black, ManaColor::Red];
static SUBS: &[baylee_core::ids::SubtypeId] = &[land::SWAMP, land::MOUNTAIN];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(9),
    oracle_id: "13ff3222-91cb-4796-a34e-899ed817694c",
    scryfall_id: "73403d04-fe97-4830-8b80-16dd1a1a6cc1",
    faces: &[FaceDef {
        name: "Badlands",
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
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Red]),
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
