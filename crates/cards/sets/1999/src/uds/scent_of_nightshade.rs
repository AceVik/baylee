//! Scent of Nightshade — {1}{B} — Instant
//! Oracle: Reveal any number of black cards in your hand. Target creature gets -X/-X until end of turn, where X is the number of cards revealed this way.
//! Set: UDS #69 — Urza's Destiny | Scryfall ID: 582468a6-0ea9-411e-a694-13977d47c877 | Oracle ID: 2e88f39d-6177-42d9-b756-144219dc0961
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCENT_OF_NIGHTSHADE,
    oracle_id = "2e88f39d-6177-42d9-b756-144219dc0961",
    scryfall_id = "582468a6-0ea9-411e-a694-13977d47c877",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Scent of Nightshade",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
