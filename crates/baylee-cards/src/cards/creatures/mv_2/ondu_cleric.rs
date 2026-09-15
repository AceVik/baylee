//! Ondu Cleric — {1}{W} — Creature — Kor Cleric Ally
//! Oracle: Whenever this creature or another Ally you control enters, you may gain life equal to the number of Allies you control.
//! Set: ZEN #30 — Zendikar | Scryfall ID: ced43447-fefc-482a-b8fa-33b9616aa532 | Oracle ID: f4232466-dd6a-49bf-be6c-95905c3ded17
// IMPLEMENTED — rally: ETB of self or another Ally you control → gain 1 life.

use crate::filters::{YOUR_ALLIES, YOUR_ALLY};
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ONDU_CLERIC,
    oracle_id = "f4232466-dd6a-49bf-be6c-95905c3ded17",
    scryfall_id = "ced43447-fefc-482a-b8fa-33b9616aa532",
    faces = &[face!(
        name = "Ondu Cleric",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::KOR,
            subtypes::creature::CLERIC,
            subtypes::creature::ALLY,
        ],
        power = Some(1),
        toughness = Some(1),
    )],
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::EntersBattlefield(&YOUR_ALLIES),
        &[Effect::MayDo {
            effects: &[Effect::GainLife {
                amount: Amount::CountOf {
                    filter: &YOUR_ALLY,
                    zone: ZoneSel::Battlefield,
                },
            }],
        }]
    )],
);

// Engine-level test lives in baylee-engine (cleric_rally_gains_life):
// own ETB triggers once, another Ally's ETB triggers again, non-Ally
// creatures do not trigger.
