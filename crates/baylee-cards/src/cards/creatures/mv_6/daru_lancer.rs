//! Daru Lancer — {4}{W}{W} — Creature — Human Soldier
//! Oracle: First strike
//! Oracle: Morph {2}{W}{W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #20 — Onslaught | Scryfall ID: cd888ca8-0ebe-46f0-9317-3b193ccc43fb | Oracle ID: 21edfe1a-2c53-44b3-b0d1-3dba8fb3346d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DARU_LANCER,
    oracle_id = "21edfe1a-2c53-44b3-b0d1-3dba8fb3346d",
    scryfall_id = "cd888ca8-0ebe-46f0-9317-3b193ccc43fb",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Daru Lancer",
        mana_cost = mana!("{4}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
