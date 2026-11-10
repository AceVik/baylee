//! Whipcorder — {W}{W} — Creature — Human Soldier Rebel
//! Oracle: {W}, {T}: Tap target creature.
//! Oracle: Morph {W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #60 — Onslaught | Scryfall ID: 3bf6987e-a6e4-4a88-af0b-cf3b2d2b80c7 | Oracle ID: 4a4c187e-e10b-4fd2-b70b-6401bb0595c5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WHIPCORDER,
    oracle_id = "4a4c187e-e10b-4fd2-b70b-6401bb0595c5",
    scryfall_id = "3bf6987e-a6e4-4a88-af0b-cf3b2d2b80c7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Whipcorder",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::SOLDIER,
            subtypes::creature::REBEL
        ],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
