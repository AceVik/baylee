//! Weatherseed Treefolk — {2}{G}{G}{G} — Creature — Treefolk
//! Oracle: Trample
//! Oracle: When this creature dies, return it to its owner's hand.
//! Set: ULG #116 — Urza's Legacy | Scryfall ID: f42cce45-3b6a-43e2-8329-68c30135c5c1 | Oracle ID: 4e9b878b-80a2-433b-941b-92d854e83d58
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WEATHERSEED_TREEFOLK,
    oracle_id = "4e9b878b-80a2-433b-941b-92d854e83d58",
    scryfall_id = "f42cce45-3b6a-43e2-8329-68c30135c5c1",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Weatherseed Treefolk",
        mana_cost = mana!("{2}{G}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TREEFOLK],
        power = Some(5),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::ReturnToHand {
            target: TargetSpec::ThisObject,
        }],
    )],
);
