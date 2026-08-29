//! Godless Shrine — (no cost) — Land — PLAINS SWAMP
//! Oracle: ({T}: Add {W} or {B}.)
//! Godless Shrine enters the battlefield tapped unless you pay 2 life.
//! Set: FDN #281 — Foundations | Scryfall ID: 8fbd1ae0-3d4c-492a-a1ea-85a95fa3d7b6 | Oracle ID: 73864fcc-1bde-4bc0-831e-2b93e546e417
// IMPLEMENTED — shockland (pay 2 life or enters tapped; intrinsic mana via subtypes).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Coverage, EnterModifier,
    FaceDef, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, land};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(61),
    oracle_id: "73864fcc-1bde-4bc0-831e-2b93e546e417",
    scryfall_id: "8fbd1ae0-3d4c-492a-a1ea-85a95fa3d7b6",
    faces: &[FaceDef {
        name: "Godless Shrine",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::land::PLAINS, subtypes::land::SWAMP],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[EnterModifier::TappedOrPayLife(2)],
        abilities: &[],
        castable_from_hand: true,
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[],
};

#[cfg(test)]
mod tests {}
