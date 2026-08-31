//! Scrubland — (no cost) — Land — PLAINS SWAMP
//! Oracle: ({T}: Add {W} or {B}.)
//! Set: VMA #313 — Vintage Masters | Scryfall ID: 9d471e36-a3ab-4a96-ba4b-8eca921ea37a | Oracle ID: c8d95ca8-7d12-4072-aeaf-e20f248c7e39
// IMPLEMENTED — two-color mana choice.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, land};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static COLORS: &[ManaColor] = &[ManaColor::White, ManaColor::Black];
static SUBS: &[baylee_core::ids::SubtypeId] = &[land::PLAINS, land::SWAMP];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(140),
    oracle_id: "c8d95ca8-7d12-4072-aeaf-e20f248c7e39",
    scryfall_id: "9d471e36-a3ab-4a96-ba4b-8eca921ea37a",
    faces: &[FaceDef {
        name: "Scrubland",
        types: TypeSet::LAND,
        subtypes: SUBS,
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddManaChoice {
            colors: COLORS,
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
