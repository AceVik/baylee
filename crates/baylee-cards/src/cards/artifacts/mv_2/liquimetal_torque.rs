//! Liquimetal Torque — {2} — Artifact
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Target nonland permanent becomes an artifact in addition to its other types until end of turn.
//! Set: MH2 #228 — Modern Horizons 2 | Scryfall ID: 13c6101a-da40-4785-8ccb-4e779bbbdb55 | Oracle ID: b7d4b7dd-fbb1-4ca3-875f-ef13a95e66ad
// IMPLEMENTED — mana rock + timed type change.

use baylee_cards_dsl::prelude::*;

card! {
    index: 86,
    oracle_id: "b7d4b7dd-fbb1-4ca3-875f-ef13a95e66ad",
    scryfall_id: "13c6101a-da40-4785-8ccb-4e779bbbdb55",
    faces: &[face! {
        name: "Liquimetal Torque",
        mana_cost: baylee_core::mana!("{2}"),
        types: TypeSet::ARTIFACT,
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(Cost::TAP, &[Effect::CreateContinuousEffect {
                layer: Layer::Type,
                filter: &Filter::NONLAND,
                modifier: Modifier::AddType(TypeSet::ARTIFACT),
                duration: Duration::UntilEndOfTurn,
            }], target: Some(TargetSpec::Object(&Filter::NONLAND))),
    ],
}
