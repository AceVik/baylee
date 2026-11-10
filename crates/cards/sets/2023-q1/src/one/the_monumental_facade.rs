//! The Monumental Facade — (no cost) — Land — Sphere
//! Oracle: This land enters with two oil counters on it.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Remove an oil counter from this land: Put an oil counter on target artifact or creature you control. Activate only as a sorcery.
//! Set: ONE #255 — Phyrexia: All Will Be One | Scryfall ID: d6785057-0d06-4f91-b45f-c05f7c4e2b19 | Oracle ID: 63e8d282-d038-4a8c-a0cb-f51ddf87d8ea

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THE_MONUMENTAL_FACADE,
    oracle_id = "63e8d282-d038-4a8c-a0cb-f51ddf87d8ea",
    scryfall_id = "d6785057-0d06-4f91-b45f-c05f7c4e2b19",
    faces = &[face!(
        name = "The Monumental Facade",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::SPHERE],
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: counters::OIL,
            amount: Amount::Fixed(2)
        }],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!(
                TapSelf,
                RemoveCounterSelf {
                    kind: counters::OIL,
                    n: 1
                }
            ),
            &[Effect::AddCounter {
                kind: counters::OIL,
                amount: Amount::Fixed(1)
            }],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::ControlledByYou,
                Filter::HasType(TypeSet::ARTIFACT.union(TypeSet::CREATURE))
            ]))),
            timing = ActivationTiming::SorcerySpeed
        ),
    ],
);
