//! Inspirit, Flagship Vessel — {4} — Legendary Artifact — Spacecraft
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)
//! Oracle: 1+ | At the beginning of combat on your turn, put your choice of a +1/+1 counter or two charge counters on up to one other target artifact.
//! Oracle: 8+ | Flying
//! Oracle: Other artifacts you control have hexproof and indestructible.
//! Set: EOC #39 — Edge of Eternities Commander | Scryfall ID: 46900ec7-eb18-45c4-8e90-a48b665cfdee | Oracle ID: 554df866-3dbb-4811-8573-6033481591aa
// PARTIAL — the 1+ combat trigger counter choice and the 8+ static grant
// (conditional layer effect with counter threshold) need conditional
// continuous effects (M2+). Station itself needs power-scaled cost
// counters (M2+).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(73),
    oracle_id: "554df866-3dbb-4811-8573-6033481591aa",
    scryfall_id: "46900ec7-eb18-45c4-8e90-a48b665cfdee",
    faces: &[FaceDef {
        name: "Inspirit, Flagship Vessel",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::LEGENDARY,
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
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("station + conditional 8+ effects (M2+)"),
    abilities: &[],
};

#[cfg(test)]
mod tests {}
