//! Triassic Egg — {4} — Artifact
//! Oracle: {3}, {T}: Put a hatchling counter on this artifact.
//! Oracle: Sacrifice this artifact: Choose one. Activate only if there are two or more hatchling counters on this artifact.
//! Oracle: • You may put a creature card from your hand onto the battlefield.
//! Oracle: • Return target creature card from your graveyard to the battlefield.
//! Set: ME4 #235 — Masters Edition IV | Scryfall ID: 7ccd4ea9-1552-4fcc-a443-0718b81dc6a6 | Oracle ID: 4f27abe8-3e98-4bb8-afdb-e4b718d83032
// IMPLEMENTED — hatchling counters gate one modal sacrifice activation, usable while tapped.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRIASSIC_EGG,
    oracle_id = "4f27abe8-3e98-4bb8-afdb-e4b718d83032",
    scryfall_id = "7ccd4ea9-1552-4fcc-a443-0718b81dc6a6",
    faces = &[face!(
        name = "Triassic Egg",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        activated!(
            cost!("{3}", TapSelf),
            &[Effect::AddCounter {
                kind: counters::HATCHLING,
                amount: Amount::Fixed(1)
            }]
        ),
        modal_activated!(
            cost!(SacrificeSelf),
            &[
                mode!(&[Effect::PutFromHandOntoBattlefield {
                    tapped: false,
                    filter: &Filter::CREATURE,
                    mana_value: None,
                    optional: true,
                }]),
                mode!(
                    &[Effect::reanimate(TargetSpec::CardInGraveyard(
                        &Filter::CREATURE,
                        PlayerRel::You
                    ))],
                    targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
                        &Filter::CREATURE,
                        PlayerRel::You
                    )))
                ),
            ],
            condition = Some(Condition::CountersOnSelf(counters::HATCHLING, 2))
        ),
    ],
);
