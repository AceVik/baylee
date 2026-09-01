//! Forest — (no cost) — Basic Land — Forest
//! Oracle: ({T}: Add {G}.)
//! Set: TRK #325 — Star Trek | Scryfall ID: dce15387-4114-4b3e-91aa-5b42b45c44ac | Oracle ID: b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6
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
    index: CardIndex::new(55),
    oracle_id: "b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6",
    scryfall_id: "dce15387-4114-4b3e-91aa-5b42b45c44ac",
    faces: &[FaceDef {
        name: "Forest",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::BASIC,
        subtypes: &[land::FOREST],
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Green]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::mana(ManaColor::Green, 1)],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
    }],
    ..CardDef::DEFAULT
};
