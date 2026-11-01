//! Blinkmoth Urn — {5} — Artifact
//! Oracle: At the beginning of each player's first main phase, if this artifact is untapped, that player adds {C} for each artifact they control.
//! Set: C18 #197 — Commander 2018 | Scryfall ID: 3879faf0-6e39-4275-82f6-7fb5c70fc6ce | Oracle ID: 08497e8f-3521-48e4-a1e7-c192b485d891
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLINKMOTH_URN,
    oracle_id = "08497e8f-3521-48e4-a1e7-c192b485d891",
    scryfall_id = "3879faf0-6e39-4275-82f6-7fb5c70fc6ce",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Blinkmoth Urn",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::FirstMain,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::AddManaAmountFor {
            who: PlayerRel::ActivePlayer,
            color: ManaColor::Colorless,
            amount: Amount::ControlledCount {
                who: PlayerRel::Recipient,
                filter: &Filter::ARTIFACT,
            },
        }],
        condition = Some(Condition::SourceMatches(&Filter::Untapped)),
    )],
);
