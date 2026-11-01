//! Wall of Deceit — {1}{U} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: {3}: Turn this creature face down.
//! Oracle: Morph {U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: JVC #5 — Duel Decks Anthology: Jace vs. Chandra | Scryfall ID: 6eab8bfb-6e46-44b7-8fb1-72bb1c1d04cc | Oracle ID: 73714a40-5e8d-4aea-87fe-5aa0db53d6ff
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_DECEIT,
    oracle_id = "73714a40-5e8d-4aea-87fe-5aa0db53d6ff",
    scryfall_id = "6eab8bfb-6e46-44b7-8fb1-72bb1c1d04cc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Wall of Deceit",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
