//! Wishmonger — {3}{W} — Creature — Unicorn Monger
//! Oracle: {2}: Target creature gains protection from the color of its controller's choice until end of turn. Any player may activate this ability.
//! Set: MMQ #57 — Mercadian Masques | Scryfall ID: 5a0d8834-109e-4235-a145-75edc43da0ec | Oracle ID: f68b90af-9502-4d99-aadf-2bfcea8a655a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WISHMONGER,
    oracle_id = "f68b90af-9502-4d99-aadf-2bfcea8a655a",
    scryfall_id = "5a0d8834-109e-4235-a145-75edc43da0ec",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Wishmonger",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::UNICORN, subtypes::creature::MONGER],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
