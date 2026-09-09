//! Thief of Blood — {4}{B}{B} — Creature — Vampire
//! Oracle: Flying
//! Oracle: As this creature enters, remove all counters from all permanents. This creature enters with a +1/+1 counter on it for each counter removed this way.
//! Set: CMA #71 — Commander Anthology | Scryfall ID: 1625be56-a8e9-44f3-a213-b758bffd447f | Oracle ID: 97d61346-bd53-4eb8-a920-6ae0382eb20d
// IMPLEMENTED — drains all counters on the battlefield into +1/+1
// counters on itself (ETB trigger approximates the as-it-enters timing).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card! {
    index: 169,
    oracle_id: "97d61346-bd53-4eb8-a920-6ae0382eb20d",
    scryfall_id: "1625be56-a8e9-44f3-a213-b758bffd447f",
    faces: &[face! {
        name: "Thief of Blood",
        mana_cost: baylee_core::mana!("{4}{B}{B}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::VAMPIRE],
        power: Some(1),
        toughness: Some(1),
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::FLYING,
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::DrainAllCountersIntoSelf])],
}
