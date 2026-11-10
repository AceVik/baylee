//! Sewers of Estark — {2}{B}{B} — Instant
//! Oracle: Choose target creature. If it's attacking, it can't be blocked this turn. If it's blocking, prevent all combat damage that would be dealt this combat by it and each creature it's blocking.
//! Set: PHPR #2 — HarperPrism Book Promos | Scryfall ID: b0da11d4-3603-4f59-8f61-7204bf04e165 | Oracle ID: 2482f98a-eadf-450d-9e73-fc441b572862

use baylee_cards_dsl::prelude::*;

static BLOCKER_AND_BLOCKED: Filter = Filter::Or(&[Filter::Reference, Filter::BlockedByReference]);

card!(
    index = index::SEWERS_OF_ESTARK,
    oracle_id = "2482f98a-eadf-450d-9e73-fc441b572862",
    scryfall_id = "b0da11d4-3603-4f59-8f61-7204bf04e165",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sewers of Estark",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::IfTargetMatches {
                filter: &Filter::Attacking,
                then: &[Effect::ModifyTargets {
                    modifier: Modifier::AddKeyword(KeywordSet::UNBLOCKABLE),
                    duration: Duration::UntilEndOfTurn,
                }],
            },
            Effect::IfTargetMatches {
                filter: &Filter::Blocking,
                then: &[Effect::continuous(
                    &BLOCKER_AND_BLOCKED,
                    Modifier::PreventDamageFromIt,
                    Duration::UntilEndOfCombat,
                )],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
