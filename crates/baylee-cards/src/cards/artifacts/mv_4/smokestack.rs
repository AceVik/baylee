//! Smokestack — {4} — Artifact
//! Oracle: At the beginning of your upkeep, you may put a soot counter on this artifact.
//! Oracle: At the beginning of each player's upkeep, that player sacrifices a permanent of their choice for each soot counter on this artifact.
//! Set: USG #309 — Urza's Saga | Scryfall ID: 6a37ab2d-094f-431e-878f-93aef0360413 | Oracle ID: fda61506-ed79-47fd-badd-639185297913
// PARTIAL: sacrificing a permanent for each soot counter on this artifact is not in Effect; puts a soot counter on itself on your upkeep if you choose and each player sacrifices one permanent at the beginning of their upkeep.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SMOKESTACK,
    oracle_id = "fda61506-ed79-47fd-badd-639185297913",
    scryfall_id = "6a37ab2d-094f-431e-878f-93aef0360413",
    coverage = Coverage::Partial(
        "sacrificing a permanent for each soot counter on this artifact is not in Effect; puts a soot counter on itself on your upkeep if you choose and each player sacrifices one permanent at the beginning of their upkeep",
    ),
    faces = &[face!(
        name = "Smokestack",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::MayDo {
                effects: &[Effect::AddCounterFilter {
                    filter: &Filter::This,
                    kind: counters::SOOT,
                    amount: Amount::Fixed(1),
                }],
            }],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::SacrificeFilter {
                who: PlayerRel::ActivePlayer,
                filter: &Filter::Any,
            }],
        ),
    ],
);
