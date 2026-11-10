//! Tower of Coireall — {2} — Artifact
//! Oracle: {T}: Target creature can't be blocked by Walls this turn.
//! Set: DRK #113 — The Dark | Scryfall ID: 64c19977-ac7d-4ce7-925c-33a7503420f5 | Oracle ID: f9afad36-933f-4d18-acb7-32e5cb5c8161

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const WALL: Filter = Filter::HasSubtype(subtypes::creature::WALL);

card!(
    index = index::TOWER_OF_COIREALL,
    oracle_id = "f9afad36-933f-4d18-acb7-32e5cb5c8161",
    scryfall_id = "64c19977-ac7d-4ce7-925c-33a7503420f5",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tower of Coireall",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::ModifyTargets {
            modifier: Modifier::CantBeBlockedBy(&WALL),
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
