//! Ransack — {3}{U} — Sorcery
//! Oracle: Look at the top five cards of target player's library. Put any number of them on the bottom of that library in any order and the rest on top of the library in any order.
//! Set: S99 #47 — Starter 1999 | Scryfall ID: 97b8db82-4632-40fd-b1e2-7172ddf8d6de | Oracle ID: 5b4a400b-435d-4140-984b-65f034c6a9fc
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RANSACK,
    oracle_id = "5b4a400b-435d-4140-984b-65f034c6a9fc",
    scryfall_id = "97b8db82-4632-40fd-b1e2-7172ddf8d6de",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ransack",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::ScryFor {
            player: PlayerRel::Chosen,
            amount: Amount::Fixed(5)
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    ),],
);
