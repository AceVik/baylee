//! Bamboozle — {2}{U} — Sorcery
//! Oracle: Target player reveals the top four cards of their library. You choose two of those cards and put them into that player's graveyard. Put the rest on top of their library in any order.
//! Set: ODY #68 — Odyssey | Scryfall ID: 44012bb8-17b7-4b50-a796-662ef09bfc29 | Oracle ID: a1eccd85-d243-4d27-bf49-d3ac816159e4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BAMBOOZLE,
    oracle_id = "a1eccd85-d243-4d27-bf49-d3ac816159e4",
    scryfall_id = "44012bb8-17b7-4b50-a796-662ef09bfc29",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Bamboozle",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
