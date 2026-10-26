//! Reviving Vapors — {2}{W}{U} — Instant
//! Oracle: Reveal the top three cards of your library and put one of them into your hand. You gain life equal to that card's mana value. Put all other cards revealed this way into your graveyard.
//! Set: UMA #205 — Ultimate Masters | Scryfall ID: b7a6c902-4705-4210-8e5e-08d6a6f67573 | Oracle ID: 57104a79-cfb6-4330-8ea1-7726534ada56
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REVIVING_VAPORS,
    oracle_id = "57104a79-cfb6-4330-8ea1-7726534ada56",
    scryfall_id = "b7a6c902-4705-4210-8e5e-08d6a6f67573",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(
        name = "Reviving Vapors",
        mana_cost = mana!("{2}{W}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
