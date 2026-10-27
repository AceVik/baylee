//! Gloomdrifter — {3}{B} — Creature — Zombie Minion
//! Oracle: Flying
//! Oracle: Threshold — As long as there are seven or more cards in your graveyard, this creature has "When this creature enters, nonblack creatures get -2/-2 until end of turn."
//! Set: TOR #61 — Torment | Scryfall ID: baaf8c24-3f5e-4ea5-941b-40ad1299af39 | Oracle ID: cfc1156c-b787-4212-96e0-a8e3609069b7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GLOOMDRIFTER,
    oracle_id = "cfc1156c-b787-4212-96e0-a8e3609069b7",
    scryfall_id = "baaf8c24-3f5e-4ea5-941b-40ad1299af39",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Gloomdrifter",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::MINION],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
