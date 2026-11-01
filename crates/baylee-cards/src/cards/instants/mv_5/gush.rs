//! Gush — {4}{U} — Instant
//! Oracle: You may return two Islands you control to their owner's hand rather than pay this spell's mana cost.
//! Oracle: Draw two cards.
//! Set: JVC #27 — Duel Decks Anthology: Jace vs. Chandra | Scryfall ID: d4a3a921-3b7f-474c-b8c3-67a1a6ba5cc1 | Oracle ID: 16d8ee99-8ec8-429f-9ba7-818a74b6f910
// PARTIAL

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: alternative cost of returning two Islands you control to their owner's hand (`cast_wizard::paid_as_an_alternative_cost` only supports `PayLife` and `ExileFromHand`).
card!(
    index = index::GUSH,
    oracle_id = "16d8ee99-8ec8-429f-9ba7-818a74b6f910",
    scryfall_id = "d4a3a921-3b7f-474c-b8c3-67a1a6ba5cc1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "cast_wizard::paid_as_an_alternative_cost only supports PayLife and ExileFromHand, not returning two Islands to hand",
    ),
    faces = &[face!(
        name = "Gush",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::draw(2)])],
);
