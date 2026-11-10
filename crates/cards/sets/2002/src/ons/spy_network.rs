//! Spy Network — {U} — Instant
//! Oracle: Look at target player's hand, the top card of that player's library, and any face-down creatures they control. Look at the top four cards of your library, then put them back in any order.
//! Set: ONS #115 — Onslaught | Scryfall ID: 8a4bed3f-845c-4822-b8af-8b511dce6fe2 | Oracle ID: 81a5893a-764c-41ea-ba81-346b3c7558a4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPY_NETWORK,
    oracle_id = "81a5893a-764c-41ea-ba81-346b3c7558a4",
    scryfall_id = "8a4bed3f-845c-4822-b8af-8b511dce6fe2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Spy Network",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
