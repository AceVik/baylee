//! Vindicate — {1}{W}{B} — Sorcery
//! Oracle: Destroy target permanent.
//! Set: MH2 #294 — Modern Horizons 2 | Scryfall ID: 683c4e13-525c-45c9-8832-bfe67965c34e | Oracle ID: 63c1ac21-e3d8-40c2-8c09-3f31c52992ef
// IMPLEMENTED — destroy any target permanent (can't be regenerated).

use baylee_cards_dsl::prelude::*;

card! {
    index: 184,
    oracle_id: "63c1ac21-e3d8-40c2-8c09-3f31c52992ef",
    scryfall_id: "683c4e13-525c-45c9-8832-bfe67965c34e",
    faces: &[face! {
        name: "Vindicate",
        mana_cost: baylee_core::mana!("{1}{W}{B}"),
        types: TypeSet::SORCERY,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::Destroy {
            target: TargetSpec::Object(&Filter::Any),
        }], targets: Some(TargetReq::one(TargetSpec::Object(&Filter::Any))))],
}

// Engine-level coverage via s4 scenario tests: the chosen permanent is
// destroyed (battlefield → graveyard).
