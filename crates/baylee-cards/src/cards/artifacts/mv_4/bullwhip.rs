//! Bullwhip — {4} — Artifact
//! Oracle: {2}, {T}: This artifact deals 1 damage to target creature. That creature attacks this turn if able.
//! Set: STH #132 — Stronghold | Scryfall ID: ff312dd7-456c-4ce5-9614-e2cbe3c2219c | Oracle ID: d4e7a9f5-5228-4e41-8f58-6e7453fe68dc
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BULLWHIP,
    oracle_id = "d4e7a9f5-5228-4e41-8f58-6e7453fe68dc",
    scryfall_id = "ff312dd7-456c-4ce5-9614-e2cbe3c2219c",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Bullwhip",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{2}", TapSelf),
        &[
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Object(&Filter::CREATURE),
            },
            Effect::ModifyTargets {
                modifier: Modifier::AttacksEachCombat,
                duration: Duration::UntilEndOfTurn,
            },
        ],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
