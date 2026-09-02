//! Emeria's Call // Emeria, Shattered Skyclave — {4}{W}{W}{W} — Sorcery // Land
//! Set: ZNR #12 — Zendikar Rising | Scryfall ID: c470539a-9cc7-4175-8f7c-c982b6072b6d | Oracle ID: 6ec2a242-9068-4ee2-8ac8-8341cc570f56
//! Face: Emeria's Call — {4}{W}{W}{W} — Sorcery
//! Face: Emeria, Shattered Skyclave —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 469,
    oracle_id: "6ec2a242-9068-4ee2-8ac8-8341cc570f56",
    scryfall_id: "c470539a-9cc7-4175-8f7c-c982b6072b6d",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Emeria's Call",
        mana_cost: baylee_core::mana!("{4}{W}{W}{W}"),
        types: TypeSet::SORCERY,
    },
    face! {
        name: "Emeria, Shattered Skyclave",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
