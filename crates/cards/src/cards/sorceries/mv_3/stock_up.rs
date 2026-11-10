//! Stock Up — {2}{U} — Sorcery
//! Oracle: Look at the top five cards of your library. Put two of them into your hand and the rest on the bottom of your library in any order.
//! Set: DFT #67 — Aetherdrift | Scryfall ID: 0a786855-6eb4-42c0-a528-4842db46809d | Oracle ID: 2251e34e-4ce8-4452-9dc6-d8cce8583996
// IMPLEMENTED — look at top 5, put 2 into hand, rest on bottom in any order.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STOCK_UP,
    oracle_id = "2251e34e-4ce8-4452-9dc6-d8cce8583996",
    scryfall_id = "0a786855-6eb4-42c0-a528-4842db46809d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Stock Up",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::LookAtTopPick {
        count: Amount::Fixed(5),
        pick: 2,
        random: false,
    }])],
);
