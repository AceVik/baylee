//! Words of Wind — {2}{U} — Enchantment
//! Oracle: {1}: The next time you would draw a card this turn, each player returns a permanent they control to its owner's hand instead.
//! Set: ONS #122 — Onslaught | Scryfall ID: 5595a57a-a76c-467b-afaf-5affffc24f35 | Oracle ID: 523d3e3d-8fdf-44f8-868d-7b00995e4577
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WORDS_OF_WIND,
    oracle_id = "523d3e3d-8fdf-44f8-868d-7b00995e4577",
    scryfall_id = "5595a57a-a76c-467b-afaf-5affffc24f35",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Words of Wind",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
