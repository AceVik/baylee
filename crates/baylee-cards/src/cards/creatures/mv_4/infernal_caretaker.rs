//! Infernal Caretaker — {3}{B} — Creature — Human Cleric
//! Oracle: Morph {3}{B} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, return all Zombie cards from all graveyards to their owners' hands.
//! Set: LGN #76 — Legions | Scryfall ID: 8a028a30-6242-4d87-9501-d1826ecb69b0 | Oracle ID: bd310727-1091-4474-8d9e-ce56f7dac7c1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INFERNAL_CARETAKER,
    oracle_id = "bd310727-1091-4474-8d9e-ce56f7dac7c1",
    scryfall_id = "8a028a30-6242-4d87-9501-d1826ecb69b0",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Infernal Caretaker",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
