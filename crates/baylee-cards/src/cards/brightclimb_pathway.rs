//! Brightclimb Pathway // Grimclimb Pathway — (no cost) — Land // Land
//! Oracle: Brightclimb Pathway: {T}: Add {W}. // Grimclimb Pathway: {T}: Add {B}.
//! Set: ZNR #259 — Zendikar Rising | Scryfall ID: d24c3d51-795d-4c01-a34a-3280fccd2d78 | Oracle ID: 1c633e02-95ef-445e-b4e0-fbfbc5ed9cc9
// IMPLEMENTED — MDFC land-face choice on play (CR 712.4a) + per-face
// mana abilities.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Effect,
    FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static BACK_MANA: &[AbilityDef] = &[AbilityDef::Activated {
    cost: Cost::TAP,
    effects: &[Effect::mana(ManaColor::Black, 1)],
    target: None,
    timing: ActivationTiming::InstantSpeed,
    mana_ability: true,
    zone: ActivationZone::Battlefield,
}];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(16),
    oracle_id: "1c633e02-95ef-445e-b4e0-fbfbc5ed9cc9",
    scryfall_id: "d24c3d51-795d-4c01-a34a-3280fccd2d78",
    faces: &[
        FaceDef {
            name: "Brightclimb Pathway",
            types: TypeSet::LAND,
            ..FaceDef::DEFAULT
        },
        FaceDef {
            name: "Grimclimb Pathway",
            types: TypeSet::LAND,
            abilities: BACK_MANA,
            ..FaceDef::DEFAULT
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::mana(ManaColor::White, 1)],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
    }],
    ..CardDef::DEFAULT
};
