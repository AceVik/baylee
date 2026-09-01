//! Arcane Sanctum — (no cost) — Land
//! Oracle: Arcane Sanctum enters the battlefield tapped.
//! {T}: Add White, Blue, or Black.
//! Set: C16 #281 — Commander 2016 | Scryfall ID: c75eeb97-3249-4762-84b0-387f27fb255f | Oracle ID: 7d7cf15c-06b9-4062-a1eb-32614c458a3b
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
    index: CardIndex::new(5),
    oracle_id: "7d7cf15c-06b9-4062-a1eb-32614c458a3b",
    scryfall_id: "c75eeb97-3249-4762-84b0-387f27fb255f",
    faces: &[FaceDef {
        name: "Arcane Sanctum",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::Tapped],
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::mana_choice(&[
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
        ])],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
    }],
    ..CardDef::DEFAULT
};
