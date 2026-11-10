//! Scent of Brine — {1}{U} — Instant
//! Oracle: Reveal any number of blue cards in your hand. Counter target spell unless its controller pays {1} for each card revealed this way.
//! Set: UDS #45 — Urza's Destiny | Scryfall ID: d117bf8d-23ec-4f9d-99d0-3a990c5f7075 | Oracle ID: c22b5968-adea-4f74-877d-8ac2ee452769
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCENT_OF_BRINE,
    oracle_id = "c22b5968-adea-4f74-877d-8ac2ee452769",
    scryfall_id = "d117bf8d-23ec-4f9d-99d0-3a990c5f7075",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Scent of Brine",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
