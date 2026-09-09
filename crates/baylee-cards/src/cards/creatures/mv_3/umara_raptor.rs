//! Umara Raptor — {2}{U} — Creature — Bird Ally
//! Oracle: Flying
//! Oracle: Whenever this creature or another Ally you control enters, you may put a +1/+1 counter on this creature.
//! Set: ZEN #75 — Zendikar | Scryfall ID: 6049cc80-1faa-48bf-897e-fefe5a8e7ab2 | Oracle ID: a58ee84f-1d9c-4924-b7b1-14a9b2ba3b98
// IMPLEMENTED — flying + rally counter on self.

use crate::filters::YOUR_ALLIES;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card! {
    index: 177,
    oracle_id: "a58ee84f-1d9c-4924-b7b1-14a9b2ba3b98",
    scryfall_id: "6049cc80-1faa-48bf-897e-fefe5a8e7ab2",
    faces: &[face! {
        name: "Umara Raptor",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::BIRD, creature::ALLY],
        power: Some(1),
        toughness: Some(1),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::FLYING,
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::EntersBattlefield(&YOUR_ALLIES), &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }])],
}
