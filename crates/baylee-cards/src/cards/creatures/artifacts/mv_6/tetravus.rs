//! Tetravus — {6} — Artifact Creature — Construct
//! Oracle: Flying
//! Oracle: This creature enters with three +1/+1 counters on it.
//! Oracle: At the beginning of your upkeep, you may remove any number of +1/+1 counters from this creature. If you do, create that many 1/1 colorless Tetravite artifact creature tokens. They each have flying and "This token can't be enchanted."
//! Oracle: At the beginning of your upkeep, you may exile any number of tokens created with this creature. If you do, put that many +1/+1 counters on this creature.
//! Set: ME4 #233 — Masters Edition IV | Scryfall ID: c1b83a15-1d0b-4fc6-b7cb-fcd0063bfa7d | Oracle ID: 85255c26-4e74-4cf0-91a6-78ddba5abdc6

use crate::tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TETRAVUS,
    oracle_id = "85255c26-4e74-4cf0-91a6-78ddba5abdc6",
    scryfall_id = "c1b83a15-1d0b-4fc6-b7cb-fcd0063bfa7d",
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tetravus",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(1),
        toughness = Some(1),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(3)
        }],
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::MayRemoveCountersSelfCreateTokens {
                kind: CounterKind::P1P1,
                token: &tokens::TETRAVITE_1_1_COLORLESS_ARTIFACT_FLYING,
            }]
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::MayExileTokensCreatedWithThisForCounters {
                kind: CounterKind::P1P1,
            }]
        ),
    ],
);
