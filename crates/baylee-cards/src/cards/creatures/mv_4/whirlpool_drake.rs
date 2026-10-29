//! Whirlpool Drake — {3}{U} — Creature — Drake
//! Oracle: Flying
//! Oracle: When this creature enters, shuffle the cards from your hand into your library, then draw that many cards.
//! Oracle: When this creature dies, shuffle the cards from your hand into your library, then draw that many cards.
//! Set: APC #34 — Apocalypse | Scryfall ID: 6e866093-89a3-458d-8ebc-de805ef7885e | Oracle ID: aef9fcf0-b755-40f6-821d-836ae1b5497a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WHIRLPOOL_DRAKE,
    oracle_id = "aef9fcf0-b755-40f6-821d-836ae1b5497a",
    scryfall_id = "6e866093-89a3-458d-8ebc-de805ef7885e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Whirlpool Drake",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAKE],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
