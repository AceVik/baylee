//! Swamp — (no cost) — Basic Land — Swamp
//! Oracle: ({T}: Add {B}.)
//! Set: TRK #321 — Star Trek | Scryfall ID: b7387103-1df1-4fd0-9e91-1544509792c7 | Oracle ID: 56719f6a-1a6c-4c0a-8d21-18f7d7350b68
// IMPLEMENTED — basic land mana ability.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Effect,
    FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, land};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(162),
    oracle_id: "56719f6a-1a6c-4c0a-8d21-18f7d7350b68",
    scryfall_id: "b7387103-1df1-4fd0-9e91-1544509792c7",
    faces: &[FaceDef {
        name: "Swamp",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::BASIC,
        subtypes: &[land::SWAMP],
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::mana(ManaColor::Black, 1)],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
    }],
    ..CardDef::DEFAULT
};
