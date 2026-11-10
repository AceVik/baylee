//! Suppress — {2}{B} — Sorcery
//! Oracle: Target player exiles all cards from their hand face down. At the beginning of the end step of that player's next turn, that player returns those cards to their hand.
//! Set: APC #52 — Apocalypse | Scryfall ID: 642eefde-8727-44ff-9e04-373abfcd0679 | Oracle ID: b39651ad-1c05-45f8-b036-536c4d7324c0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SUPPRESS,
    oracle_id = "b39651ad-1c05-45f8-b036-536c4d7324c0",
    scryfall_id = "642eefde-8727-44ff-9e04-373abfcd0679",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Suppress",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
