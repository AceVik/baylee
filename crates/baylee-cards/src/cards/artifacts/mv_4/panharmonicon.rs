//! Panharmonicon — {4} — Artifact
//! Oracle: If an artifact or creature entering causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.
//! Set: 2X2 #310 — Double Masters 2022 | Scryfall ID: 998d0cc8-ca2a-41c3-ab65-d05c26ab8278 | Oracle ID: 76678885-3674-443d-b9a2-2a460cf6aac0
// IMPLEMENTED — ETB-trigger multiplication for your permanents.

use baylee_cards_dsl::prelude::*;

card! {
    index: 110,
    oracle_id: "76678885-3674-443d-b9a2-2a460cf6aac0",
    scryfall_id: "998d0cc8-ca2a-41c3-ab65-d05c26ab8278",
    faces: &[face! {
        name: "Panharmonicon",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::ARTIFACT,
    }],
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Replacement(
        ReplacementRule::TriggerMultiplier {
            source_filter: &Filter::ControlledByYou,
            event: TriggerEventKind::EntersBattlefield,
        },
    )],
}

// Engine-level coverage in baylee-engine s6 tests: a rally trigger
// fires twice with Panharmonicon on the battlefield.
