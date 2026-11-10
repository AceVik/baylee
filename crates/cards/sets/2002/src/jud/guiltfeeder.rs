//! Guiltfeeder — {3}{B}{B} — Creature — Horror
//! Oracle: Fear (This creature can't be blocked except by artifact creatures and/or black creatures.)
//! Oracle: Whenever this creature attacks and isn't blocked, defending player loses 1 life for each card in their graveyard.
//! Set: CLB #756 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 8276b064-60ba-4079-b790-e2eea831e35e | Oracle ID: 0285bd20-f49e-48f7-8c8c-960f9fbe4d34
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GUILTFEEDER,
    oracle_id = "0285bd20-f49e-48f7-8c8c-960f9fbe4d34",
    scryfall_id = "8276b064-60ba-4079-b790-e2eea831e35e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Guiltfeeder",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HORROR],
        power = Some(0),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
