//! Tolarian Winds — {1}{U} — Instant
//! Oracle: Discard all the cards in your hand, then draw that many cards.
//! Set: USG #104 — Urza's Saga | Scryfall ID: 5c29399d-0a59-4dcb-9bd4-f31eea3f39f9 | Oracle ID: 686cce6e-18ec-45b0-8d2e-74fa353a905e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TOLARIAN_WINDS,
    oracle_id = "686cce6e-18ec-45b0-8d2e-74fa353a905e",
    scryfall_id = "5c29399d-0a59-4dcb-9bd4-f31eea3f39f9",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tolarian Winds",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[
        Effect::DiscardHand {
            who: PlayerRel::You,
        },
        Effect::DrawCardsFor {
            who: PlayerRel::You,
            amount: Amount::ThisWay(ThisWay::Discarded),
        },
    ]),],
);
