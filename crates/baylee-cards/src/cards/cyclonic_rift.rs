//! Cyclonic Rift — {1}{U} — Instant
//! Oracle: Return target nonland permanent you don't control to its owner's hand.
//! Oracle: Overload {6}{U} (You may cast this spell for its overload cost. If you do, change "target" in its text to "each.")
//! Set: RVR #40 — Ravnica Remastered | Scryfall ID: dfb7c4b9-f2f4-4d4e-baf2-86551c8150fe | Oracle ID: d75b9c82-1b49-4c3e-a1b5-aeef57d6644b
// IMPLEMENTED — modal spell: single-target bounce or overloaded mass bounce
// (choose cast mode in the wizard).

static NOT_MINE: Filter = Filter::And(&[Filter::Not(&Filter::ControlledByYou), Filter::NONLAND]);

static NORMAL_EFFECTS: &[Effect] = &[Effect::ReturnToHand {
    target: TargetSpec::Object(&NOT_MINE),
}];
static OVERLOAD_EFFECTS: &[Effect] = &[Effect::ReturnAllToHand {
    filter: &Filter::NONLAND,
    opponents_only: true,
}];

use baylee_cards_dsl::prelude::*;

card! {
    index: 29,
    oracle_id: "d75b9c82-1b49-4c3e-a1b5-aeef57d6644b",
    scryfall_id: "dfb7c4b9-f2f4-4d4e-baf2-86551c8150fe",
    faces: &[face! {
        name: "Cyclonic Rift",
        mana_cost: baylee_core::mana!("{1}{U}"),
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::ModalSpell {
        modes: &[
            mode!(NORMAL_EFFECTS, target: Some(TargetSpec::Object(&NOT_MINE))),
            mode!(OVERLOAD_EFFECTS, cost_override: Some(baylee_core::mana!("{6}{U}"))),
        ],
    }],
}

// Engine-level coverage in baylee-engine s7 tests: both modes resolve.
