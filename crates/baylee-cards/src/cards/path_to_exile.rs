//! Path to Exile — {W} — Instant
//! Oracle: Exile target creature. Its controller may search their library for a basic land card, put that card onto the battlefield tapped, then shuffle.
//! Set: MSC #141 — Marvel Super Heroes Commander | Scryfall ID: 95ca89ea-1200-4bb4-ae4b-af35d3ccd35b | Oracle ID: d683d985-9888-4d21-8b5f-69e69ce4a03b
// IMPLEMENTED — exile + optional basic-land ramp for the creature's controller.

use baylee_cards_dsl::prelude::*;

card! {
    index: 112,
    oracle_id: "d683d985-9888-4d21-8b5f-69e69ce4a03b",
    scryfall_id: "95ca89ea-1200-4bb4-ae4b-af35d3ccd35b",
    faces: &[face! {
        name: "Path to Exile",
        mana_cost: baylee_core::mana!("{W}"),
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[
            Effect::Exile {
                target: TargetSpec::Object(&Filter::CREATURE),
            },
            Effect::OptionalBasicLandSearchFor {
                player: PlayerRel::ControllerOfTarget,
            },
        ], targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))],
}
