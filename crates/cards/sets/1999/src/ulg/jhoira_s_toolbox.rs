//! Jhoira's Toolbox — {2} — Artifact Creature — Insect
//! Oracle: {2}: Regenerate target artifact creature.
//! Set: ULG #128 — Urza's Legacy | Scryfall ID: edb38309-c02c-496c-894f-786a2f6e3d1c | Oracle ID: a08475a6-1fac-4a3f-b343-aed16a61bc16
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const ARTIFACT_CREATURE: Filter = Filter::And(&[Filter::ARTIFACT, Filter::CREATURE]);

card!(
    index = index::JHOIRA_S_TOOLBOX,
    oracle_id = "a08475a6-1fac-4a3f-b343-aed16a61bc16",
    scryfall_id = "edb38309-c02c-496c-894f-786a2f6e3d1c",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Jhoira's Toolbox",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::INSECT],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{2}"),
        &[Effect::Regenerate {
            target: TargetSpec::Object(&ARTIFACT_CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&ARTIFACT_CREATURE))),
    )],
);
