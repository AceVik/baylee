//! Watery Grave — (no cost) — Land — ISLAND SWAMP
//! Oracle: ({T}: Add {U} or {B}.)
//! Watery Grave enters the battlefield tapped unless you pay 2 life.
//! Set: FDN #284 — Foundations | Scryfall ID: 5525d6a6-e532-4047-9da4-bfae7927fecc | Oracle ID: fc9ec820-4245-4a96-b009-5308a818ca58
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
    index: CardIndex::new(189),
    oracle_id: "fc9ec820-4245-4a96-b009-5308a818ca58",
    scryfall_id: "5525d6a6-e532-4047-9da4-bfae7927fecc",
    faces: &[FaceDef {
        name: "Watery Grave",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::ISLAND, subtypes::land::SWAMP],
        enter_modifiers: &[EnterModifier::TappedOrPayLife(2)],
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::mana_choice(&[ManaColor::Blue, ManaColor::Black])],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
    }],
    ..CardDef::DEFAULT
};
