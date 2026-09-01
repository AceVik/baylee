//! Fetid Heath — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! {1}, {T}: Add two mana in any combination of {White} and/or {Black}.
//! Set: SHM #272 — Shadowmoor | Scryfall ID: f465ded8-0d38-42ac-bafc-a12185013c5d | Oracle ID: 42bf259d-4bb9-49c3-b4ec-223dca62f4d6
// IMPLEMENTED — filter land (colorless tap + {1},{T} for two combination mana).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, Coverage,
    Effect, FaceDef, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(49),
    oracle_id: "42bf259d-4bb9-49c3-b4ec-223dca62f4d6",
    scryfall_id: "f465ded8-0d38-42ac-bafc-a12185013c5d",
    faces: &[FaceDef {
        name: "Fetid Heath",
        types: TypeSet::LAND,
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::mana(ManaColor::Colorless, 1)],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
        AbilityDef::Activated {
            cost: Cost {
                mana: baylee_core::mana!("{1}"),
                parts: &[baylee_cards_dsl::CostPart::TapSelf],
            },
            effects: &[Effect::mana_combination(
                &[ManaColor::White, ManaColor::Black],
                Amount::Fixed(2),
            )],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
    ],
    ..CardDef::DEFAULT
};
