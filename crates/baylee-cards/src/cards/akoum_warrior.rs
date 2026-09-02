//! Akoum Warrior // Akoum Teeth — {5}{R} — Creature — Minotaur Warrior // Land
//! Set: ZNR #134 — Zendikar Rising | Scryfall ID: d8ed0335-daa6-4dbe-a94d-4d56c8cfd093 | Oracle ID: afedce7b-0e18-40ad-a26a-1933fddb560d
//! Face: Akoum Warrior — {5}{R} — Creature — Minotaur Warrior
//! Face: Akoum Teeth —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 215,
    oracle_id: "afedce7b-0e18-40ad-a26a-1933fddb560d",
    scryfall_id: "d8ed0335-daa6-4dbe-a94d-4d56c8cfd093",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    faces: &[
    face! {
        name: "Akoum Warrior",
        mana_cost: baylee_core::mana!("{5}{R}"),
        types: TypeSet::CREATURE,
        subtypes: &[subtypes::creature::MINOTAUR, subtypes::creature::WARRIOR],
        power: Some(4),
        toughness: Some(5),
    },
    face! {
        name: "Akoum Teeth",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
