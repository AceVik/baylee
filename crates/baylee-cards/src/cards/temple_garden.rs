//! Temple Garden — (no cost) — Land — FOREST PLAINS
//! Oracle: ({T}: Add {G} or {W}.)
//! Temple Garden enters the battlefield tapped unless you pay 2 life.
//! Set: FDN #283 — Foundations | Scryfall ID: b9b0589d-f327-46a7-8bac-06b7654c547a | Oracle ID: f413a83d-a40d-434c-b20a-4c707c0527fa
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
    index: CardIndex::new(167),
    oracle_id: "f413a83d-a40d-434c-b20a-4c707c0527fa",
    scryfall_id: "b9b0589d-f327-46a7-8bac-06b7654c547a",
    faces: &[FaceDef {
        name: "Temple Garden",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::FOREST, subtypes::land::PLAINS],
        enter_modifiers: &[EnterModifier::TappedOrPayLife(2)],
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddManaChoice {
            colors: &[ManaColor::Green, ManaColor::White],
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
