//! Earth King's Lieutenant — {G}{W} — Creature — Human Soldier Ally
//! Oracle: Trample
//! Oracle: When this creature enters, put a +1/+1 counter on each other Ally creature you control.
//! Oracle: Whenever another Ally you control enters, put a +1/+1 counter on this creature.
//! Set: TLA #217 — Avatar: The Last Airbender | Scryfall ID: 4533d155-5c56-41a5-9d76-2d1414ac47c9 | Oracle ID: 9da9248d-1201-447f-b6c2-2b64af4f71c4
// IMPLEMENTED — trample + ETB team counters + rally counter on self.

use crate::filters::ANOTHER_ALLY;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card! {
    index: 37,
    oracle_id: "9da9248d-1201-447f-b6c2-2b64af4f71c4",
    scryfall_id: "4533d155-5c56-41a5-9d76-2d1414ac47c9",
    faces: &[face! {
        name: "Earth King's Lieutenant",
        mana_cost: baylee_core::mana!("{G}{W}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::HUMAN, creature::SOLDIER, creature::ALLY],
        power: Some(1),
        toughness: Some(1),
    }],
    color_identity: ColorSet::from_slice(&[Color::Green, Color::White]),
    keywords: KeywordSet::TRAMPLE,
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::AddCounterFilter {
                filter: &ANOTHER_ALLY,
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }]),
        triggered!(Trigger::EntersBattlefield(&ANOTHER_ALLY), &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }]),
    ],
}
