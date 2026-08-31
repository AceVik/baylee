//! Mox Opal — {0} — Legendary Artifact
//! Oracle: Metalcraft — {T}: Add one mana of any color. Activate only if you control three or more artifacts.
//! Set: 2XM #275 — Double Masters | Scryfall ID: 56001a36-126b-4c08-af98-a6cc4d84210e | Oracle ID: de2440de-e948-4811-903c-0bbe376ff64d
// IMPLEMENTED — metalcraft: the mana ability activates only with 3+
// artifacts under your control (ActivationCondition::ControlCount).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    ALL_MANA_COLORS, AbilityDef, ActivationCondition, ActivationTiming, ActivationZone, Amount,
    CardDef, CommanderRule, Cost, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(98),
    oracle_id: "de2440de-e948-4811-903c-0bbe376ff64d",
    scryfall_id: "56001a36-126b-4c08-af98-a6cc4d84210e",
    faces: &[FaceDef {
        name: "Mox Opal",
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::LEGENDARY,
        ..FaceDef::DEFAULT
    }],
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::ActivatedConditional {
        cost: Cost::TAP,
        effects: &[Effect::AddManaChoice {
            colors: ALL_MANA_COLORS,
            amount: Amount::Fixed(1),
            combination: false,
        }],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
        condition: ActivationCondition::ControlCount(&Filter::HasType(TypeSet::ARTIFACT), 3),
    }],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
