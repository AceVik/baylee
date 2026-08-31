//! Temple Garden — (no cost) — Land — FOREST PLAINS
//! Oracle: ({T}: Add {G} or {W}.)
//! Temple Garden enters the battlefield tapped unless you pay 2 life.
//! Set: FDN #283 — Foundations | Scryfall ID: b9b0589d-f327-46a7-8bac-06b7654c547a | Oracle ID: f413a83d-a40d-434c-b20a-4c707c0527fa
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
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
