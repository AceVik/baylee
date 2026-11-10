//! Ironfist Crusher — {4}{W} — Creature — Human Soldier
//! Oracle: This creature can block any number of creatures.
//! Oracle: Morph {3}{W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #42 — Onslaught | Scryfall ID: c7284e32-de54-4c83-a7de-7b249c47319a | Oracle ID: ba4c9337-4cdf-44d8-849b-ffd808e5407d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::IRONFIST_CRUSHER,
    oracle_id = "ba4c9337-4cdf-44d8-849b-ffd808e5407d",
    scryfall_id = "c7284e32-de54-4c83-a7de-7b249c47319a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Ironfist Crusher",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
