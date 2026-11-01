//! Fatal Push — {B} — Instant
//! Oracle: Destroy target creature if it has mana value 2 or less.
//! Oracle: Revolt — Destroy that creature if it has mana value 4 or less instead if a permanent left the battlefield under your control this turn.
//! Set: 2XM #93 — Double Masters | Scryfall ID: 6e9d8fe4-fd9b-4923-92bf-7dd6b8fa02e7 | Oracle ID: 16437a83-be52-44cd-a768-a767c9347eb2
// IMPLEMENTED — the mana-value check is made on resolution, with Revolt raising it.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FATAL_PUSH,
    oracle_id = "16437a83-be52-44cd-a768-a767c9347eb2",
    scryfall_id = "6e9d8fe4-fd9b-4923-92bf-7dd6b8fa02e7",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fatal Push",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::IfCondition {
            condition: Condition::Revolt,
            then: &[Effect::IfTargetMatches {
                filter: &Filter::CmcAtMost(4),
                then: &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
            }],
            otherwise: &[Effect::IfTargetMatches {
                filter: &Filter::CmcAtMost(2),
                then: &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
