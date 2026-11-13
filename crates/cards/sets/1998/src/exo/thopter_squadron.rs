//! Thopter Squadron — {5} — Artifact Creature — Thopter
//! Oracle: Flying
//! Oracle: This creature enters with three +1/+1 counters on it.
//! Oracle: {1}, Remove a +1/+1 counter from this creature: Create a 1/1 colorless Thopter artifact creature token with flying. Activate only as a sorcery.
//! Oracle: {1}, Sacrifice another Thopter: Put a +1/+1 counter on this creature. Activate only as a sorcery.
//! Set: TPR #233 — Tempest Remastered | Scryfall ID: 5fd13ac2-9642-4d59-a5c6-cc13c86991f0 | Oracle ID: f1482ca4-610b-48fe-ac23-4526637bb72a
// IMPLEMENTED — all printed abilities are represented.

use crate::generated_tokens::THOPTER_ARTIFACT_1_1_FLYING;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ANOTHER_THOPTER_YOU_CONTROL: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::creature::THOPTER),
    Filter::ControlledByYou,
    Filter::Another,
]);

card!(
    index = index::THOPTER_SQUADRON,
    oracle_id = "f1482ca4-610b-48fe-ac23-4526637bb72a",
    scryfall_id = "5fd13ac2-9642-4d59-a5c6-cc13c86991f0",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Thopter Squadron",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::THOPTER],
        power = Some(0),
        toughness = Some(0),
        keywords = KeywordSet::FLYING,
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(3),
        }],
    ),],
    abilities = &[
        activated!(
            cost!(
                "{1}",
                RemoveCounterSelf {
                    kind: CounterKind::P1P1,
                    n: 1,
                }
            ),
            &[Effect::CreateToken {
                token: &THOPTER_ARTIFACT_1_1_FLYING,
            }],
            timing = ActivationTiming::SorcerySpeed,
        ),
        activated!(
            cost!("{1}", Sacrifice(&ANOTHER_THOPTER_YOU_CONTROL)),
            &[Effect::AddCounterSource {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
