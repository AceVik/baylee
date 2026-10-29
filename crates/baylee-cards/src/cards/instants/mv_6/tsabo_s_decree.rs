//! Tsabo's Decree — {5}{B} — Instant
//! Oracle: Choose a creature type. Target player reveals their hand and discards all creature cards of that type. Then destroy all creatures of that type that player controls. They can't be regenerated.
//! Set: INV #129 — Invasion | Scryfall ID: 0c1a0ebd-1add-49e6-b5e6-5b26abb1de88 | Oracle ID: 4e0de4d9-7a30-4a2b-94ba-8239baad2a0a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TSABO_S_DECREE,
    oracle_id = "4e0de4d9-7a30-4a2b-94ba-8239baad2a0a",
    scryfall_id = "0c1a0ebd-1add-49e6-b5e6-5b26abb1de88",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Tsabo's Decree",
        mana_cost = mana!("{5}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
