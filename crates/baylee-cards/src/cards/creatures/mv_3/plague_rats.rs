//! Plague Rats — {2}{B} — Creature — Rat
//! Oracle: Plague Rats's power and toughness are each equal to the number of creatures named Plague Rats on the battlefield.
//! Set: 5ED #188 — Fifth Edition | Scryfall ID: c99fd75c-4b41-411f-92b0-ca3b220946b5 | Oracle ID: 16cf1cf9-6900-406c-a5b4-0e5750f530e0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PLAGUE_RATS,
    oracle_id = "16cf1cf9-6900-406c-a5b4-0e5750f530e0",
    scryfall_id = "c99fd75c-4b41-411f-92b0-ca3b220946b5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Plague Rats",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::RAT],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
