//! Karn, Silver Golem — {5} — Legendary Artifact Creature — Golem
//! Oracle: Whenever Karn blocks or becomes blocked, it gets -4/+4 until end of turn.
//! Oracle: {1}: Target noncreature artifact becomes an artifact creature with power and toughness each equal to its mana value until end of turn.
//! Set: USG #298 — Urza's Saga | Scryfall ID: 811a0988-2900-426c-9413-8f1778d99678 | Oracle ID: 8a7c6391-020a-4d7a-8abb-702631bbee56
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KARN_SILVER_GOLEM,
    oracle_id = "8a7c6391-020a-4d7a-8abb-702631bbee56",
    scryfall_id = "811a0988-2900-426c-9413-8f1778d99678",
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Karn, Silver Golem",
        mana_cost = mana!("{5}"),
        supertypes = SupertypeSet::LEGENDARY,
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[
        triggered!(
            Trigger::BlocksOrBecomesBlockedByOneOrMore(&Filter::CREATURE),
            &[Effect::continuous(
                &Filter::This,
                Modifier::ModifyPT(-4, 4),
                Duration::UntilEndOfTurn,
            )],
        ),
        activated!(
            cost!("{1}"),
            &[Effect::ModifyTargets {
                modifier: Modifier::AnimateNoncreatureArtifact,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::ARTIFACT,
                Filter::NONCREATURE,
            ]))),
        ),
    ],
);
