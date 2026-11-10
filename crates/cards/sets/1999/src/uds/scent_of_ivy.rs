//! Scent of Ivy — {G} — Instant
//! Oracle: Reveal any number of green cards in your hand. Target creature gets +X/+X until end of turn, where X is the number of cards revealed this way.
//! Set: UDS #120 — Urza's Destiny | Scryfall ID: a56b4894-b959-4d00-b631-95d26eb85a4e | Oracle ID: b6db81e5-b7ca-45dc-a72d-1ff12935a3e4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCENT_OF_IVY,
    oracle_id = "b6db81e5-b7ca-45dc-a72d-1ff12935a3e4",
    scryfall_id = "a56b4894-b959-4d00-b631-95d26eb85a4e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Scent of Ivy",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
