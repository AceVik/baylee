//! Ondu Cleric — {1}{W} — Creature — Human Cleric Ally
//! Oracle: Whenever this creature or another Ally you control enters, you may gain life equal to the number of Allies you control.
//! Set: ZEN #30 — Zendikar | Scryfall ID: ced43447-fefc-482a-b8fa-33b9616aa532 | Oracle ID: f4232466-dd6a-49bf-be6c-95905c3ded17
// IMPLEMENTED — rally: ETB of self or another Ally you control → gain 1 life.

use crate::filters::YOUR_ALLIES;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{self, creature};

// "Ondu Cleric or another Ally … under your control"
static ALLIES_YOU_CONTROL: Filter =
    Filter::And(&[Filter::HasSubtype(creature::ALLY), Filter::ControlledByYou]);

card! {
    index: 104,
    oracle_id: "f4232466-dd6a-49bf-be6c-95905c3ded17",
    scryfall_id: "ced43447-fefc-482a-b8fa-33b9616aa532",
    faces: &[face! {
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
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::EntersBattlefield(&YOUR_ALLIES), &[Effect::GainLife {
            amount: Amount::CountOf {
                filter: &ALLIES_YOU_CONTROL,
                zone: ZoneSel::Battlefield,
            },
        }])],
}

// Engine-level test lives in baylee-engine (cleric_rally_gains_life):
// own ETB triggers once, another Ally's ETB triggers again, non-Ally
// creatures do not trigger.
