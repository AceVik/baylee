//! Crude Rampart — {3}{W} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Morph {4}{W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #17 — Onslaught | Scryfall ID: af5d1be2-d6ae-4820-aa01-62f261b0f110 | Oracle ID: c9823e5f-d0f8-4d28-8b3b-55f94ea9d76f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CRUDE_RAMPART,
    oracle_id = "c9823e5f-d0f8-4d28-8b3b-55f94ea9d76f",
    scryfall_id = "af5d1be2-d6ae-4820-aa01-62f261b0f110",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Crude Rampart",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(4),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
