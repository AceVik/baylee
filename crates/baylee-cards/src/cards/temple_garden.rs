//! Temple Garden — (no cost) — Land — FOREST PLAINS
//! Oracle: ({T}: Add {G} or {W}.)
//! Temple Garden enters the battlefield tapped unless you pay 2 life.
//! Set: FDN #283 — Foundations | Scryfall ID: b9b0589d-f327-46a7-8bac-06b7654c547a | Oracle ID: f413a83d-a40d-434c-b20a-4c707c0527fa
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
    index: CardIndex::new(167),
    oracle_id: "f413a83d-a40d-434c-b20a-4c707c0527fa",
    scryfall_id: "b9b0589d-f327-46a7-8bac-06b7654c547a",
    faces: &[FaceDef {
        name: "Temple Garden",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::land::FOREST, subtypes::land::PLAINS],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[EnterModifier::TappedOrPayLife(2)],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
        delve: false,
        convoke: false,
        cost_reduction: None,
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
