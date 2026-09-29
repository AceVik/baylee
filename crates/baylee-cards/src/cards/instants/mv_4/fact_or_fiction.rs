//! Fact or Fiction — {3}{U} — Instant
//! Oracle: Reveal the top five cards of your library. An opponent separates those cards into two piles. Put one pile into your hand and the other into your graveyard.
//! Set: FRC #41 — Reality Fracture Commander | Scryfall ID: eb1a25c5-1918-40f3-b54f-5d31cf6b4eb2 | Oracle ID: 437b2dab-15e0-4b9a-a204-58622d37a3b3

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FACT_OR_FICTION,
    oracle_id = "437b2dab-15e0-4b9a-a204-58622d37a3b3",
    scryfall_id = "eb1a25c5-1918-40f3-b54f-5d31cf6b4eb2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fact or Fiction",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::RevealAndSeparate { count: 5 }])],
);
