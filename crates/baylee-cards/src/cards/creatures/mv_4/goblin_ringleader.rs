//! Goblin Ringleader — {3}{R} — Creature — Goblin
//! Oracle: Haste (This creature can attack and {T} as soon as it comes under your control.)
//! Oracle: When this creature enters, reveal the top four cards of your library. Put all Goblin cards revealed this way into your hand and the rest on the bottom of your library in any order.
//! Set: M20 #143 — Core Set 2020 | Scryfall ID: 6911af4e-4abd-4cf9-8a86-ae5d680c6f12 | Oracle ID: 4100e486-0d27-436c-8429-76bc2c1a26ab
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_RINGLEADER,
    oracle_id = "4100e486-0d27-436c-8429-76bc2c1a26ab",
    scryfall_id = "6911af4e-4abd-4cf9-8a86-ae5d680c6f12",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Goblin Ringleader",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
