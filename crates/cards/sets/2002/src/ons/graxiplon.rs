//! Graxiplon — {5}{U} — Creature — Beast
//! Oracle: This creature can't be blocked unless defending player controls three or more creatures that share a creature type.
//! Set: ONS #86 — Onslaught | Scryfall ID: 0c16e565-0b7f-46b1-a091-64c47c923a9f | Oracle ID: b81ed78a-1bcc-4a98-8d06-6c9fdff0a8d3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GRAXIPLON,
    oracle_id = "b81ed78a-1bcc-4a98-8d06-6c9fdff0a8d3",
    scryfall_id = "0c16e565-0b7f-46b1-a091-64c47c923a9f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Graxiplon",
        mana_cost = mana!("{5}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
