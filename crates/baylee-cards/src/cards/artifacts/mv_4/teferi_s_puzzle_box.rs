//! Teferi's Puzzle Box — {4} — Artifact
//! Oracle: At the beginning of each player's draw step, that player puts the cards in their hand on the bottom of their library in any order, then draws that many cards.
//! Set: 9ED #312 — Ninth Edition | Scryfall ID: 415e81e9-ca65-4bc0-8aab-d905d58fe6cc | Oracle ID: 37abcc92-9466-47ea-9e0b-5eda2eb62c8e
// PARTIAL: triggers at the beginning of your draw step to let you put cards from your hand on the bottom of your library and draw that many cards; triggering on each player's draw step and requiring the active player to bottom their entire hand is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEFERI_S_PUZZLE_BOX,
    oracle_id = "37abcc92-9466-47ea-9e0b-5eda2eb62c8e",
    scryfall_id = "415e81e9-ca65-4bc0-8aab-d905d58fe6cc",
    coverage = Coverage::Partial(
        "triggers at the beginning of your draw step to let you put cards from your hand on the bottom of your library and draw that many cards; triggering on each player's draw step and requiring the active player to bottom their entire hand is not yet modeled",
    ),
    faces = &[face!(
        name = "Teferi's Puzzle Box",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Draw,
            whose: PlayerRel::You,
        },
        &[
            Effect::PutAnyFromHandOnBottom,
            Effect::DrawCards {
                amount: Amount::ThisWay(ThisWay::PutIntoLibraryFromHand),
            },
        ],
    )],
);
