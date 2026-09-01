//! Underground Sea — (no cost) — Land — ISLAND SWAMP
//! Oracle: ({T}: Add {B} or {B}.)
//! Set: VMA #323 — Vintage Masters | Scryfall ID: 26cee543-6eab-494e-a803-33a5d48d7d74 | Oracle ID: 4b22be3a-8ce1-47d1-b82e-6c3ccfb0548b
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

static COLORS: &[ManaColor] = &[ManaColor::Blue, ManaColor::Black];
static SUBS: &[baylee_core::ids::SubtypeId] = &[land::ISLAND, land::SWAMP];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(178),
    oracle_id: "4b22be3a-8ce1-47d1-b82e-6c3ccfb0548b",
    scryfall_id: "26cee543-6eab-494e-a803-33a5d48d7d74",
    faces: &[FaceDef {
        name: "Underground Sea",
        types: TypeSet::LAND,
        subtypes: SUBS,
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Black]),
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
