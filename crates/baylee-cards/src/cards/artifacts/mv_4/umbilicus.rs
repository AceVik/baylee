//! Umbilicus — {4} — Artifact
//! Oracle: At the beginning of each player's upkeep, that player may pay 2 life. If they don't, they return a permanent they control to its owner's hand.
//! Set: USG #312 — Urza's Saga | Scryfall ID: 29a6db88-a11d-49b4-8692-28b24d23f3c7 | Oracle ID: c9132e72-d016-4cb7-8662-a16b7d932bef
// PARTIAL: paying 2 life to decline returning a permanent you control to its owner's hand is not in Effect; at the beginning of each player's upkeep, that player returns a permanent they control to its owner's hand.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UMBILICUS,
    oracle_id = "c9132e72-d016-4cb7-8662-a16b7d932bef",
    scryfall_id = "29a6db88-a11d-49b4-8692-28b24d23f3c7",
    coverage = Coverage::Partial(
        "paying 2 life to decline returning a permanent you control to its owner's hand is not in Effect; at the beginning of each player's upkeep, that player returns a permanent they control to its owner's hand",
    ),
    faces = &[face!(
        name = "Umbilicus",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::ReturnChosenToHand {
            who: PlayerRel::ActivePlayer,
            filter: &Filter::Any,
        }],
    ),],
);
