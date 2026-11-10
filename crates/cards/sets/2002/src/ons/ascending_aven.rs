//! Ascending Aven — {2}{U}{U} — Creature — Bird Soldier
//! Oracle: Flying
//! Oracle: This creature can block only creatures with flying.
//! Oracle: Morph {2}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #68 — Onslaught | Scryfall ID: bd8b17df-615c-4cc1-af1a-2fc35a985af9 | Oracle ID: 67b4a4bc-0b02-4cef-b5c8-41c839eaf0cd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ASCENDING_AVEN,
    oracle_id = "67b4a4bc-0b02-4cef-b5c8-41c839eaf0cd",
    scryfall_id = "bd8b17df-615c-4cc1-af1a-2fc35a985af9",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Ascending Aven",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::SOLDIER],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
