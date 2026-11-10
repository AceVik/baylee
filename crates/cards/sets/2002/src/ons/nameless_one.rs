//! Nameless One — {3}{U} — Creature — Wizard Avatar
//! Oracle: Nameless One's power and toughness are each equal to the number of Wizards on the battlefield.
//! Oracle: Morph {2}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #100 — Onslaught | Scryfall ID: 79cf3535-3f80-4b76-aad3-dd851e6885a6 | Oracle ID: bd8d0498-53f9-484c-a59a-78daca468859
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NAMELESS_ONE,
    oracle_id = "bd8d0498-53f9-484c-a59a-78daca468859",
    scryfall_id = "79cf3535-3f80-4b76-aad3-dd851e6885a6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Nameless One",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WIZARD, subtypes::creature::AVATAR],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
