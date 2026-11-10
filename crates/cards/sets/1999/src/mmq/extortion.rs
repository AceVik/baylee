//! Extortion — {3}{B}{B} — Sorcery
//! Oracle: Look at target player's hand and choose up to two cards from it. That player discards those cards.
//! Set: MMQ #135 — Mercadian Masques | Scryfall ID: a66742db-4750-49ce-ad05-b825af7222c4 | Oracle ID: 8841fdb9-0565-4ef5-814c-293c14242741
// PARTIAL

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: choosing up to two cards from a privately looked-at hand to discard (`RevealHandDiscard` reveals the hand to all players and chooses one card).
card!(
    index = index::EXTORTION,
    oracle_id = "8841fdb9-0565-4ef5-814c-293c14242741",
    scryfall_id = "a66742db-4750-49ce-ad05-b825af7222c4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "LookAtChosenHand reveals the targeted player's hand privately to you; choosing up to two cards from it to discard is not supported",
    ),
    faces = &[face!(
        name = "Extortion",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::LookAtChosenHand],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
