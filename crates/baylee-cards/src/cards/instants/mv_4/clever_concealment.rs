//! Clever Concealment — {2}{W}{W} — Instant
//! Oracle: Convoke (Your creatures can help cast this spell. Each creature you tap while casting this spell pays for {1} or one mana of that creature's color.)
//! Oracle: Any number of target nonland permanents you control phase out. (Treat them and anything attached to them as though they don't exist until your next turn.)
//! Set: MSC #125 — Marvel Super Heroes Commander | Scryfall ID: 41d45a8a-ea1d-4fbc-86d2-5d6340f3b639 | Oracle ID: 42bb7ea9-f6e4-4551-8d93-3b1eae84b865
// IMPLEMENTED — convoke (generic {1} per tapped creature; the
// colored-mana option is a payment refinement) + mass phase-out.

static YOUR_NONLAND_PERMANENTS: Filter = Filter::And(&[Filter::NONLAND, Filter::ControlledByYou]);

use baylee_cards_dsl::prelude::*;

card! {
    index: 22,
    oracle_id: "42bb7ea9-f6e4-4551-8d93-3b1eae84b865",
    scryfall_id: "41d45a8a-ea1d-4fbc-86d2-5d6340f3b639",
    faces: &[face! {
        name: "Clever Concealment",
        mana_cost: baylee_core::mana!("{2}{W}{W}"),
        types: TypeSet::INSTANT,
        convoke: true,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::PhaseOut {
            target: Some(TargetSpec::Object(&YOUR_NONLAND_PERMANENTS)),
        }], targets: Some(TargetReq {
            spec: TargetSpec::Object(&YOUR_NONLAND_PERMANENTS),
            min: 0,
            max: u8::MAX,
            count_is_x: false,
        }))],
}
