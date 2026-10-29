//! Foothill Guide — {W} — Creature — Human Cleric
//! Oracle: Protection from Goblins
//! Oracle: Morph {W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #29 — Onslaught | Scryfall ID: 409adb7b-6dcb-4e7f-a5dd-c0adf12140a4 | Oracle ID: d91c7e21-43af-40f9-b544-f39d1e77dee9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FOOTHILL_GUIDE,
    oracle_id = "d91c7e21-43af-40f9-b544-f39d1e77dee9",
    scryfall_id = "409adb7b-6dcb-4e7f-a5dd-c0adf12140a4",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Foothill Guide",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
