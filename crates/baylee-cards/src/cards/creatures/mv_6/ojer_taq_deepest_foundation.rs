//! Ojer Taq, Deepest Foundation // Temple of Civilization — {4}{W}{W} — Legendary Creature — God // Land
//! Set: LCI #26 — The Lost Caverns of Ixalan | Scryfall ID: 1ca79dd4-67fc-496c-96fc-489b039c4932 | Oracle ID: 486bb9a5-73f1-4cec-b097-fb07ac80b72e
//! Face: Ojer Taq, Deepest Foundation — {4}{W}{W} — Legendary Creature — God
//! Face: Temple of Civilization —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 829,
    oracle_id: "486bb9a5-73f1-4cec-b097-fb07ac80b72e",
    scryfall_id: "1ca79dd4-67fc-496c-96fc-489b039c4932",
    color_identity: ColorSet::from_slice(&[Color::White]),
    commander: CommanderRule::Legendary,
    faces: &[
    face! {
        name: "Ojer Taq, Deepest Foundation",
        mana_cost: baylee_core::mana!("{4}{W}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::creature::GOD],
        power: Some(6),
        toughness: Some(6),
    },
    face! {
        name: "Temple of Civilization",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
