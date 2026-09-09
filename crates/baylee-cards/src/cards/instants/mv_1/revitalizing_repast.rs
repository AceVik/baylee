//! Revitalizing Repast // Old-Growth Grove — {B/G} — Instant // Land
//! Set: MH3 #256 — Modern Horizons 3 | Scryfall ID: 03522b6b-31ec-4126-8885-5dbb2248688b | Oracle ID: 8dd6d060-d023-48a6-85cb-7a5521b6257b
//! Face: Revitalizing Repast — {B/G} — Instant
//! Face: Old-Growth Grove —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 919,
    oracle_id: "8dd6d060-d023-48a6-85cb-7a5521b6257b",
    scryfall_id: "03522b6b-31ec-4126-8885-5dbb2248688b",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces: &[
    face! {
        name: "Revitalizing Repast",
        mana_cost: baylee_core::mana!("{B/G}"),
        types: TypeSet::INSTANT,
    },
    face! {
        name: "Old-Growth Grove",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
