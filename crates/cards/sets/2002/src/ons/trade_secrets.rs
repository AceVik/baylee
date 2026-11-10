//! Trade Secrets — {1}{U}{U} — Sorcery
//! Oracle: Target opponent draws two cards, then you draw up to four cards. That opponent may repeat this process as many times as they choose.
//! Set: CMD #64 — Commander 2011 | Scryfall ID: 58dadc78-fe87-40ac-94cb-128716d89d74 | Oracle ID: 50885640-cdf1-4c62-b3bc-f37db6ab38b5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRADE_SECRETS,
    oracle_id = "50885640-cdf1-4c62-b3bc-f37db6ab38b5",
    scryfall_id = "58dadc78-fe87-40ac-94cb-128716d89d74",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Trade Secrets",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
