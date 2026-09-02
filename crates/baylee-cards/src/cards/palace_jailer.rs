//! Palace Jailer — {2}{W}{W} — Creature — Human Soldier
//! Oracle: When this creature enters, you become the monarch.
//! Oracle: When this creature enters, exile target creature an opponent controls until an opponent becomes the monarch.
//! Set: C17 #21 — Commander 2017 | Scryfall ID: 3a8c2a84-e0f2-4611-af3d-42f4578ad4e3 | Oracle ID: 180eda7c-fca2-403b-85cd-8ffebaf9f408
// IMPLEMENTED — monarch designation (become monarch, monarch draw at end
// step, monarch-linked exile release).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static ENEMY_CREATURE: Filter = Filter::And(&[Filter::ControlledByOpponent, Filter::CREATURE]);

card! {
    index: 109,
    oracle_id: "180eda7c-fca2-403b-85cd-8ffebaf9f408",
    scryfall_id: "3a8c2a84-e0f2-4611-af3d-42f4578ad4e3",
    faces: &[face! {
        name: "Palace Jailer",
        mana_cost: baylee_core::mana!("{2}{W}{W}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::HUMAN, creature::SOLDIER],
        power: Some(2),
        toughness: Some(2),
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::BecomeMonarch]),
        triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::ExileLinked {
                target: TargetSpec::Object(&ENEMY_CREATURE),
            }], targets: Some(TargetReq::one(TargetSpec::Object(&ENEMY_CREATURE)))),
    ],
}
