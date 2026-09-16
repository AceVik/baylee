//! Arcbound Ravager — {2} — Artifact Creature — Beast
//! Oracle: Sacrifice an artifact: Put a +1/+1 counter on this creature.
//! Oracle: Modular 1 (This creature enters with a +1/+1 counter on it. When it dies, you may put its +1/+1 counters on target artifact creature.)
//! Set: MMA #198 — Modern Masters | Scryfall ID: c0c33a92-5621-40b4-a3a2-b67893edbc01 | Oracle ID: 62e7e7b1-9887-4d15-b0e5-a8ddc711bd88
// PARTIAL — sacrifice an artifact to put a +1/+1 counter on this creature; modular 1 is not in the DSL.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARCBOUND_RAVAGER,
    oracle_id = "62e7e7b1-9887-4d15-b0e5-a8ddc711bd88",
    scryfall_id = "c0c33a92-5621-40b4-a3a2-b67893edbc01",
    coverage = Coverage::Partial("modular is not written"),
    faces = &[face!(
        name = "Arcbound Ravager",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::BEAST],
        power = Some(0),
        toughness = Some(0),
    ),],
    abilities = &[
        activated!(
            cost!(Sacrifice(&Filter::YOUR_ARTIFACT)),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }]
        ),
        // NOT SUPPORTED: Modular 1 (This creature enters with a +1/+1 counter on it. When it dies, you may put its +1/+1 counters on target artifact creature.)
    ],
);
