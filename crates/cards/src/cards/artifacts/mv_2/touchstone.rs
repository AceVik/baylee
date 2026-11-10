//! Touchstone — {2} — Artifact
//! Oracle: {T}: Tap target artifact you don't control.
//! Set: WTH #161 — Weatherlight | Scryfall ID: 923afe8a-e82c-4b93-bb42-8f5073acae13 | Oracle ID: 92382bdb-84cf-4456-8ca1-8a19badae7ad
// IMPLEMENTED — Taps to tap target artifact you don't control.

use baylee_cards_dsl::prelude::*;

static NOT_YOUR_ARTIFACT: Filter =
    Filter::And(&[Filter::ARTIFACT, Filter::Not(&Filter::ControlledByYou)]);

card!(
    index = index::TOUCHSTONE,
    oracle_id = "92382bdb-84cf-4456-8ca1-8a19badae7ad",
    scryfall_id = "923afe8a-e82c-4b93-bb42-8f5073acae13",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Touchstone",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::TapTarget],
        targets = Some(TargetReq::one(TargetSpec::Object(&NOT_YOUR_ARTIFACT)))
    ),],
);
