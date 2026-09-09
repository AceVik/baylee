//! Vanishing Verse — {W}{B} — Instant
//! Oracle: Exile target monocolored permanent.
//! Set: SOC #335 — Secrets of Strixhaven Commander | Scryfall ID: 8a475868-a335-45e7-9d59-9dc4c2cea1ae | Oracle ID: 5b8f0cdf-572d-4025-b930-79291f7c35be
// IMPLEMENTED — monocolored exile removal.

use baylee_cards_dsl::prelude::*;

card! {
    index: 180,
    oracle_id: "5b8f0cdf-572d-4025-b930-79291f7c35be",
    scryfall_id: "8a475868-a335-45e7-9d59-9dc4c2cea1ae",
    faces: &[face! {
        name: "Vanishing Verse",
        mana_cost: baylee_core::mana!("{W}{B}"),
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::Exile {
            target: TargetSpec::Object(&Filter::Monocolored),
        }], targets: Some(TargetReq::one(TargetSpec::Object(&Filter::Monocolored))))],
}
