//! Watery Grave — (no cost) — Land — ISLAND SWAMP
//! Oracle: ({T}: Add {U} or {B}.)
//! Watery Grave enters the battlefield tapped unless you pay 2 life.
//! Set: FDN #284 — Foundations | Scryfall ID: 5525d6a6-e532-4047-9da4-bfae7927fecc | Oracle ID: fc9ec820-4245-4a96-b009-5308a818ca58
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
    index: CardIndex::new(189),
    oracle_id: "fc9ec820-4245-4a96-b009-5308a818ca58",
    scryfall_id: "5525d6a6-e532-4047-9da4-bfae7927fecc",
    faces: &[FaceDef {
        name: "Watery Grave",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::land::ISLAND, subtypes::land::SWAMP],
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
