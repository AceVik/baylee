//! Petravark — {3}{R} — Creature — Nightmare Beast
//! Oracle: When this creature enters, exile target land.
//! Oracle: When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.
//! Set: TOR #109 — Torment | Scryfall ID: ffc98d09-439e-426b-8403-4a3e12167336 | Oracle ID: aedf022c-6065-42cb-85bb-225ae1650a5c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PETRAVARK,
    oracle_id = "aedf022c-6065-42cb-85bb-225ae1650a5c",
    scryfall_id = "ffc98d09-439e-426b-8403-4a3e12167336",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Petravark",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::NIGHTMARE, subtypes::creature::BEAST],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
