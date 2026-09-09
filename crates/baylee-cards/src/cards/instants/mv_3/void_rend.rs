//! Void Rend — {W}{U}{B} — Instant
//! Oracle: This spell can't be countered.
//! Oracle: Destroy target nonland permanent.
//! Set: SNC #230 — Streets of New Capenna | Scryfall ID: 2daab74d-d66b-4164-aa19-24e8d5536f7d | Oracle ID: 713f16db-95ec-479e-a48c-7a69f7668d7f
// IMPLEMENTED — uncounterable single-target destroy.

use baylee_cards_dsl::prelude::*;

card! {
    index: 185,
    oracle_id: "713f16db-95ec-479e-a48c-7a69f7668d7f",
    scryfall_id: "2daab74d-d66b-4164-aa19-24e8d5536f7d",
    faces: &[face! {
        name: "Void Rend",
        mana_cost: baylee_core::mana!("{W}{U}{B}"),
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue, Color::Black]),
    keywords: KeywordSet::UNCOUNTERABLE,
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::Destroy {
            target: TargetSpec::Object(&Filter::NONLAND),
        }], targets: Some(TargetReq::one(TargetSpec::Object(&Filter::NONLAND))))],
}
