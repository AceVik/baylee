//! Life Matrix — {4} — Artifact
//! Oracle: {4}, {T}: Put a matrix counter on target creature and that creature gains "Remove a matrix counter from this creature: Regenerate this creature." Activate only during your upkeep.
//! Set: LEG #284 — Legends | Scryfall ID: c99a3abc-e2a3-4eee-8f72-b1b25dcd1d0b | Oracle ID: 0fd1ddc3-65cc-489f-81b9-6c144843fc67

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIFE_MATRIX,
    oracle_id = "0fd1ddc3-65cc-489f-81b9-6c144843fc67",
    scryfall_id = "c99a3abc-e2a3-4eee-8f72-b1b25dcd1d0b",
    faces = &[face!(
        name = "Life Matrix",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{4}", TapSelf),
        &[
            Effect::AddCounter {
                kind: counters::MATRIX,
                amount: Amount::Fixed(1)
            },
            Effect::ModifyTargets {
                modifier: Modifier::GrantActivated {
                    cost: cost!(RemoveCounterSelf {
                        kind: counters::MATRIX,
                        n: 1
                    }),
                    effects: &[Effect::regenerate(TargetSpec::ThisObject)],
                    mana_ability: false,
                },
                duration: Duration::Indefinitely
            },
        ],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
        condition = Some(Condition::All(&[
            Condition::YourTurn,
            Condition::DuringStep(StepKind::Upkeep)
        ]))
    ),],
);
