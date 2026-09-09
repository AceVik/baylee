//! Liquimetal Coating — {2} — Artifact
//! Oracle: {T}: Target permanent becomes an artifact in addition to its other types until end of turn.
//! Set: CM2 #197 — Commander Anthology Volume II | Scryfall ID: f631447c-36e3-4d82-a658-19c9767a216b | Oracle ID: f4bdc551-c2eb-4a34-a3e3-b4a017c925af
// IMPLEMENTED — timed type change (layer 4, until end of turn).

use baylee_cards_dsl::prelude::*;

card! {
    index: 85,
    oracle_id: "f4bdc551-c2eb-4a34-a3e3-b4a017c925af",
    scryfall_id: "f631447c-36e3-4d82-a658-19c9767a216b",
    faces: &[face! {
        name: "Liquimetal Coating",
        mana_cost: baylee_core::mana!("{2}"),
        types: TypeSet::ARTIFACT,
    }],
    coverage: Coverage::Implemented,
    abilities: &[activated!(Cost::TAP, &[Effect::CreateContinuousEffect {
            layer: Layer::Type,
            filter: &Filter::Any,
            modifier: Modifier::AddType(TypeSet::ARTIFACT),
            duration: Duration::UntilEndOfTurn,
        }], target: Some(TargetSpec::Object(&Filter::Any)))],
}
