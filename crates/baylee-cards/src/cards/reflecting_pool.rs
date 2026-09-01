//! Reflecting Pool — (no cost) — Land
//! Oracle: {T}: Add one mana of any type that a land you control could produce.
//! Set: CLB #358 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 18a1b3f5-473d-45ca-be0d-e67e77ba30ce | Oracle ID: 67f43ac6-2a58-4b53-b5d7-0330e2a252e2
// IMPLEMENTED — color choice from your lands' producible mana
// (colorless included when a land can produce it).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Effect,
    FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(128),
    oracle_id: "67f43ac6-2a58-4b53-b5d7-0330e2a252e2",
    scryfall_id: "18a1b3f5-473d-45ca-be0d-e67e77ba30ce",
    faces: &[FaceDef {
        name: "Reflecting Pool",
        types: TypeSet::LAND,
        ..FaceDef::DEFAULT
    }],
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::mana_land_color(true)],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
    }],
    ..CardDef::DEFAULT
};
