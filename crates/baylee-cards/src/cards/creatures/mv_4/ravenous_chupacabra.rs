//! Ravenous Chupacabra — {2}{B}{B} — Creature — Beast Horror
//! Oracle: When this creature enters, destroy target creature an opponent controls.
//! Set: MKC #136 — Murders at Karlov Manor Commander | Scryfall ID: a4dfbac0-1849-41c5-853a-1fee108d0b01 | Oracle ID: 7b459306-149b-4f43-abc1-2dd70c748c0e
// IMPLEMENTED — ETB kill on an opponent's creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static ENEMY_CREATURE: Filter = Filter::And(&[Filter::ControlledByOpponent, Filter::CREATURE]);

card! {
    index: 124,
    oracle_id: "7b459306-149b-4f43-abc1-2dd70c748c0e",
    scryfall_id: "a4dfbac0-1849-41c5-853a-1fee108d0b01",
    faces: &[face! {
        name: "Ravenous Chupacabra",
        mana_cost: baylee_core::mana!("{2}{B}{B}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::BEAST, creature::HORROR],
        power: Some(2),
        toughness: Some(2),
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::Destroy {
            target: TargetSpec::Object(&ENEMY_CREATURE),
        }], targets: Some(TargetReq::one(TargetSpec::Object(&ENEMY_CREATURE))))],
}
