//! Hallowed Fountain — (no cost) — Land — PLAINS ISLAND
//! Oracle: ({T}: Add {W} or {U}.)
//! Hallowed Fountain enters the battlefield tapped unless you pay 2 life.
//! Set: FDN #280 — Foundations | Scryfall ID: b7285986-7e08-4969-86ef-452dc5bfdd9f | Oracle ID: f1750962-a87c-49f6-b731-02ae971ac6ea
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
    index: CardIndex::new(65),
    oracle_id: "f1750962-a87c-49f6-b731-02ae971ac6ea",
    scryfall_id: "b7285986-7e08-4969-86ef-452dc5bfdd9f",
    faces: &[FaceDef {
        name: "Hallowed Fountain",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::PLAINS, subtypes::land::ISLAND],
        enter_modifiers: &[EnterModifier::TappedOrPayLife(2)],
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddManaChoice {
            colors: &[ManaColor::White, ManaColor::Blue],
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
