//! Thief of Blood — {4}{B}{B} — Creature — Vampire
//! Oracle: Flying
//! Oracle: As this creature enters, remove all counters from all permanents. This creature enters with a +1/+1 counter on it for each counter removed this way.
//! Set: CMA #71 — Commander Anthology | Scryfall ID: 1625be56-a8e9-44f3-a213-b758bffd447f | Oracle ID: 97d61346-bd53-4eb8-a920-6ae0382eb20d
// IMPLEMENTED — drains all counters on the battlefield into +1/+1
// counters on itself (ETB trigger approximates the as-it-enters timing).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(169),
    oracle_id: "97d61346-bd53-4eb8-a920-6ae0382eb20d",
    scryfall_id: "1625be56-a8e9-44f3-a213-b758bffd447f",
    faces: &[FaceDef {
        name: "Thief of Blood",
        mana_cost: baylee_core::mana!("{4}{B}{B}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::VAMPIRE],
        power: Some(1),
        toughness: Some(1),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::FLYING,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        once_per_turn: false,
        effects: &[Effect::DrainAllCountersIntoSelf],
        targets: None,
    }],
    ..CardDef::DEFAULT
};
