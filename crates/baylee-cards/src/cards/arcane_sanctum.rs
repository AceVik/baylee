//! Arcane Sanctum — (no cost) — Land
//! Oracle: Arcane Sanctum enters the battlefield tapped.
//! {T}: Add White, Blue, or Black.
//! Set: C16 #281 — Commander 2016 | Scryfall ID: c75eeb97-3249-4762-84b0-387f27fb255f | Oracle ID: 7d7cf15c-06b9-4062-a1eb-32614c458a3b
// IMPLEMENTED — 3-color tapland (ETB tapped).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Effect,
    EnterModifier, FaceDef, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(5),
    oracle_id: "7d7cf15c-06b9-4062-a1eb-32614c458a3b",
    scryfall_id: "c75eeb97-3249-4762-84b0-387f27fb255f",
    faces: &[FaceDef {
        name: "Arcane Sanctum",
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
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddManaChoice {
            colors: &[ManaColor::White, ManaColor::Blue, ManaColor::Black],
            amount: 1,
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
