//! Butcher Orgg — {4}{R}{R}{R} — Creature — Orgg
//! Oracle: You may assign this creature's combat damage divided as you choose among defending player and/or any number of creatures they control.
//! Set: ONS #192 — Onslaught | Scryfall ID: 7f2a29cf-4b2e-44c0-af73-512d6fed0dae | Oracle ID: 6656d47a-9490-46ee-a6ed-7cbcab7539c8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BUTCHER_ORGG,
    oracle_id = "6656d47a-9490-46ee-a6ed-7cbcab7539c8",
    scryfall_id = "7f2a29cf-4b2e-44c0-af73-512d6fed0dae",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Butcher Orgg",
        mana_cost = mana!("{4}{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ORGG],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
