//! Inspirit, Flagship Vessel — {U}{R}{W} — Legendary Artifact — Spacecraft
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)
//! Oracle: 1+ | At the beginning of combat on your turn, put your choice of a +1/+1 counter or two charge counters on up to one other target artifact.
//! Oracle: 8+ | Flying
//! Oracle: Other artifacts you control have hexproof and indestructible.
//! Set: EOC #2 — Edge of Eternities Commander | Scryfall ID: 46900ec7-eb18-45c4-8e90-a48b665cfdee | Oracle ID: 554df866-3dbb-4811-8573-6033481591aa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(73),
    oracle_id: "554df866-3dbb-4811-8573-6033481591aa",
    scryfall_id: "46900ec7-eb18-45c4-8e90-a48b665cfdee",
    faces: &[FaceDef {
        name: "Inspirit, Flagship Vessel",
        mana_cost: baylee_core::mana!("{U}{R}{W}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::artifact::SPACECRAFT],
        power: Some(5),
        toughness: Some(5),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Red, Color::Blue, Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Unimplemented,
    abilities: &[],
};

#[cfg(test)]
mod tests {
    // TODO(card): implement abilities + tests, see docs/card-dsl.md.
}
