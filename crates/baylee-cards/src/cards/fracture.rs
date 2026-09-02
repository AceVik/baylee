//! Fracture — {W}{B} — Instant
//! Oracle: Destroy target artifact, enchantment, or planeswalker.
//! Set: SOC #310 — Secrets of Strixhaven Commander | Scryfall ID: cba33bf7-0919-408c-8eb0-0bb9fe920c81 | Oracle ID: f21d0319-0509-4ac1-b6e3-10955a26fd7a
// IMPLEMENTED — flexible destroy.

static ARTIFACT_ENCHANTMENT_OR_WALKER: Filter =
    Filter::Or(&[Filter::ARTIFACT, Filter::ENCHANTMENT, Filter::PLANESWALKER]);

use baylee_cards_dsl::prelude::*;

card! {
    index: 56,
    oracle_id: "f21d0319-0509-4ac1-b6e3-10955a26fd7a",
    scryfall_id: "cba33bf7-0919-408c-8eb0-0bb9fe920c81",
    faces: &[face! {
        name: "Fracture",
        mana_cost: baylee_core::mana!("{W}{B}"),
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::Destroy {
            target: TargetSpec::Object(&ARTIFACT_ENCHANTMENT_OR_WALKER),
        }], targets: Some(TargetReq::one(TargetSpec::Object(
            &ARTIFACT_ENCHANTMENT_OR_WALKER,
        ))))],
}
