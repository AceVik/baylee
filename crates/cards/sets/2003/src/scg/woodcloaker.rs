//! Woodcloaker — {5}{G} — Creature — Elf
//! Oracle: Morph {2}{G}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, target creature gains trample until end of turn.
//! Set: SCG #134 — Scourge | Scryfall ID: 4aa85e09-a1cd-473d-98cd-c6a7c7aff949 | Oracle ID: 92d17a26-bf73-4b62-b826-4cbe913d5c5e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WOODCLOAKER,
    oracle_id = "92d17a26-bf73-4b62-b826-4cbe913d5c5e",
    scryfall_id = "4aa85e09-a1cd-473d-98cd-c6a7c7aff949",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Woodcloaker",
        mana_cost = mana!("{5}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
