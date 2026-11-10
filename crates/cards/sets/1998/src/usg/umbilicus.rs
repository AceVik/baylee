//! Umbilicus — {4} — Artifact
//! Oracle: At the beginning of each player's upkeep, that player may pay 2 life. If they don't, they return a permanent they control to its owner's hand.
//! Set: USG #312 — Urza's Saga | Scryfall ID: 29a6db88-a11d-49b4-8692-28b24d23f3c7 | Oracle ID: c9132e72-d016-4cb7-8662-a16b7d932bef
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UMBILICUS,
    oracle_id = "c9132e72-d016-4cb7-8662-a16b7d932bef",
    scryfall_id = "29a6db88-a11d-49b4-8692-28b24d23f3c7",
    coverage = Coverage::Implemented,
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
        &[Effect::PlayerMayPayLifeOr {
            player: PlayerRel::ActivePlayer,
            life: Amount::Fixed(2),
            effect: &Effect::ReturnChosenToHand {
                who: PlayerRel::ActivePlayer,
                filter: &Filter::Any,
            },
        }],
    ),],
);
