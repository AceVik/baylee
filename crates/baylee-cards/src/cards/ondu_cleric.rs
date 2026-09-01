//! Ondu Cleric — {1}{W} — Creature — Human Cleric Ally
//! Oracle: Whenever this creature or another Ally you control enters, you may gain life equal to the number of Allies you control.
//! Set: ZEN #30 — Zendikar | Scryfall ID: ced43447-fefc-482a-b8fa-33b9616aa532 | Oracle ID: f4232466-dd6a-49bf-be6c-95905c3ded17
// IMPLEMENTED — rally: ETB of self or another Ally you control → gain 1 life.
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

// "Ondu Cleric or another Ally … under your control"
static ALLY_ETB: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::Or(&[Filter::This, Filter::HasSubtype(creature::ALLY)]),
]);
static ALLIES_YOU_CONTROL: Filter =
    Filter::And(&[Filter::HasSubtype(creature::ALLY), Filter::ControlledByYou]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(104),
    oracle_id: "f4232466-dd6a-49bf-be6c-95905c3ded17",
    scryfall_id: "ced43447-fefc-482a-b8fa-33b9616aa532",
    faces: &[FaceDef {
        name: "Ondu Cleric",
        mana_cost: baylee_core::mana!("{1}{W}"),
        types: TypeSet::CREATURE,
        subtypes: &[
            subtypes::creature::HUMAN,
            subtypes::creature::CLERIC,
            subtypes::creature::ALLY,
        ],
        power: Some(1),
        toughness: Some(1),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&ALLY_ETB),
        once_per_turn: false,
        effects: &[Effect::GainLife {
            amount: Amount::CountOf {
                filter: &ALLIES_YOU_CONTROL,
                zone: baylee_cards_dsl::ZoneSel::Battlefield,
            },
        }],
        targets: None,
    }],
    ..CardDef::DEFAULT
};

// Engine-level test lives in baylee-engine (cleric_rally_gains_life):
// own ETB triggers once, another Ally's ETB triggers again, non-Ally
// creatures do not trigger.
