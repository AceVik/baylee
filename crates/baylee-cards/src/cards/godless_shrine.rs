//! Godless Shrine — (no cost) — Land — PLAINS SWAMP
//! Oracle: ({T}: Add {W} or {B}.)
//! Godless Shrine enters the battlefield tapped unless you pay 2 life.
//! Set: FDN #281 — Foundations | Scryfall ID: 8fbd1ae0-3d4c-492a-a1ea-85a95fa3d7b6 | Oracle ID: 73864fcc-1bde-4bc0-831e-2b93e546e417
// IMPLEMENTED — shockland (pay 2 life or enters tapped) with the
// two-colour mana ability its type line grants (CR 305.6).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, Coverage,
    Effect, EnterModifier, FaceDef, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, land};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(61),
    oracle_id: "73864fcc-1bde-4bc0-831e-2b93e546e417",
    scryfall_id: "8fbd1ae0-3d4c-492a-a1ea-85a95fa3d7b6",
    faces: &[FaceDef {
        name: "Godless Shrine",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::PLAINS, subtypes::land::SWAMP],
        enter_modifiers: &[EnterModifier::TappedOrPayLife(2)],
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddManaChoice {
            colors: &[ManaColor::White, ManaColor::Black],
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
