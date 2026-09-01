//! Tishana's Tidebinder — {2}{U} — Creature — Merfolk Wizard
//! Oracle: Flash
//! Oracle: When this creature enters, counter target activated or triggered ability. If countered, that permanent loses all abilities until end of turn.
//! Set: LCI #81 — The Lost Caverns of Ixalan | Scryfall ID: 907b3d1d-8c85-4707-80b5-c4d832df9846 | Oracle ID: 2993dc7d-723d-4a9b-94bd-4bb02a9f7243
// IMPLEMENTED — flash + counter target ability + ability suppression until EOT.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_ABILITY: Filter = Filter::Any;

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(170),
    oracle_id: "2993dc7d-723d-4a9b-94bd-4bb02a9f7243",
    scryfall_id: "907b3d1d-8c85-4707-80b5-c4d832df9846",
    faces: &[FaceDef {
        name: "Tishana's Tidebinder",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::MERFOLK, creature::WIZARD],
        power: Some(2),
        toughness: Some(2),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::FLASH,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        once_per_turn: false,
        effects: &[
            Effect::CounterTargetAbility,
            Effect::TargetSourceLosesAbilities,
        ],
        targets: Some(TargetReq::one(TargetSpec::AbilityOnStack(&ANY_ABILITY))),
    }],
    ..CardDef::DEFAULT
};
