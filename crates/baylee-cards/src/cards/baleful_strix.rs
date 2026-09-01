//! Baleful Strix — {U}{B} — Artifact Creature — Bird
//! Oracle: Flying, deathtouch
//! Oracle: When this creature enters, draw a card.
//! Set: OTC #215 — Outlaws of Thunder Junction Commander | Scryfall ID: be8439e6-f779-49f0-806a-b04995697a6a | Oracle ID: 37688720-03de-4eca-a82d-a0afe8d58adc
// IMPLEMENTED — keywords + ETB cantrip.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(10),
    oracle_id: "37688720-03de-4eca-a82d-a0afe8d58adc",
    scryfall_id: "be8439e6-f779-49f0-806a-b04995697a6a",
    faces: &[FaceDef {
        name: "Baleful Strix",
        mana_cost: baylee_core::mana!("{U}{B}"),
        types: TypeSet::CREATURE.union(TypeSet::ARTIFACT),
        subtypes: &[creature::BIRD],
        power: Some(1),
        toughness: Some(1),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Black]),
    keywords: KeywordSet::FLYING.union(KeywordSet::DEATHTOUCH),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        once_per_turn: false,
        effects: &[Effect::DrawCards {
            amount: Amount::Fixed(1),
        }],
        targets: None,
    }],
    ..CardDef::DEFAULT
};
