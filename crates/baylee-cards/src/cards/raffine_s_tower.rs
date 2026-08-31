//! Raffine's Tower — (no cost) — Land
//! Oracle: Raffine's Tower enters the battlefield tapped.
//! {T}: Add White, Blue, or Black.
//! Set: SNC #254 — Streets of New Capenna | Scryfall ID: a2c56479-4bee-4edb-80d7-4af010b7c793 | Oracle ID: 6e9ef5ef-6aed-4d3e-a59b-9e3dc8740b1b
// IMPLEMENTED — 3-color tapland (ETB tapped).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, Coverage,
    Effect, EnterModifier, FaceDef, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(122),
    oracle_id: "6e9ef5ef-6aed-4d3e-a59b-9e3dc8740b1b",
    scryfall_id: "a2c56479-4bee-4edb-80d7-4af010b7c793",
    faces: &[FaceDef {
        name: "Raffine's Tower",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::Tapped],
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddManaChoice {
            colors: &[ManaColor::White, ManaColor::Blue, ManaColor::Black],
            amount: Amount::Fixed(1),
            combination: false,
        }],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
    }],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
