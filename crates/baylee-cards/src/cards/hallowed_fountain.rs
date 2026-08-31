//! Hallowed Fountain — (no cost) — Land — PLAINS ISLAND
//! Oracle: ({T}: Add {W} or {U}.)
//! Hallowed Fountain enters the battlefield tapped unless you pay 2 life.
//! Set: FDN #280 — Foundations | Scryfall ID: b7285986-7e08-4969-86ef-452dc5bfdd9f | Oracle ID: f1750962-a87c-49f6-b731-02ae971ac6ea
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
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
