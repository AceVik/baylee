//! Agonizing Memories — {2}{B}{B} — Sorcery
//! Oracle: Look at target player's hand and choose two cards from it. Put them on top of that player's library in any order.
//! Set: 10E #126 — Tenth Edition | Scryfall ID: 28bac7a8-d3bb-4032-b30b-db7328e4d2fc | Oracle ID: 2ac9d397-10ca-43ce-bff9-45d631553486
// NOT SUPPORTED: choosing two cards from target player's hand and putting them on top of their library in any order is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AGONIZING_MEMORIES,
    oracle_id = "2ac9d397-10ca-43ce-bff9-45d631553486",
    scryfall_id = "28bac7a8-d3bb-4032-b30b-db7328e4d2fc",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "looks at target player's hand; choosing two cards from it and putting them on top of that player's library in any order is not yet modeled",
    ),
    faces = &[face!(
        name = "Agonizing Memories",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::LookAtChosenHand],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
