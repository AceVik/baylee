//! Joven's Tools — {6} — Artifact
//! Oracle: {4}, {T}: Target creature can't be blocked this turn except by Walls.
//! Set: 5ED #386 — Fifth Edition | Scryfall ID: 91c7aa8e-5873-4e18-8adb-1523c01d4e86 | Oracle ID: b1935140-b038-4a81-98f7-28d2c137124a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NON_WALL_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
]);

card!(
    index = index::JOVEN_S_TOOLS,
    oracle_id = "b1935140-b038-4a81-98f7-28d2c137124a",
    scryfall_id = "91c7aa8e-5873-4e18-8adb-1523c01d4e86",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Joven's Tools",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{4}", TapSelf),
        &[Effect::ModifyTargets {
            modifier: Modifier::CantBeBlockedBy(&NON_WALL_CREATURE),
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
