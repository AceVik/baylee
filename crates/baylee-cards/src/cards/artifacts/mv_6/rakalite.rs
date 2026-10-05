//! Rakalite — {6} — Artifact
//! Oracle: {2}: Prevent the next 1 damage that would be dealt to any target this turn. Return this artifact to its owner's hand at the beginning of the next end step.
//! Set: ME4 #223 — Masters Edition IV | Scryfall ID: 90633271-f4dc-45a5-ab5f-84321145b0fe | Oracle ID: 193c1671-328e-4f9c-836e-055f46c3aab0
// IMPLEMENTED — {2}: prevent the next 1 damage to any target, and a delayed
// trigger returns this artifact to its owner's hand at the beginning of the
// next end step.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RAKALITE,
    oracle_id = "193c1671-328e-4f9c-836e-055f46c3aab0",
    scryfall_id = "90633271-f4dc-45a5-ab5f-84321145b0fe",
    faces = &[face!(
        name = "Rakalite",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{2}"),
        &[
            Effect::PreventNextDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(1),
            },
            Effect::AtNextEndStep {
                effects: &[Effect::ReturnToHand {
                    target: TargetSpec::ThisObject,
                }],
            },
        ],
        target = Some(TargetSpec::AnyTarget)
    )],
);
