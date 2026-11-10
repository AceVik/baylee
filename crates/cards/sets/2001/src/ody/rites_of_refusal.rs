//! Rites of Refusal — {1}{U} — Instant
//! Oracle: Discard any number of cards. Counter target spell unless its controller pays {3} for each card discarded this way.
//! Set: ODY #99 — Odyssey | Scryfall ID: fa88f595-1b6f-4af0-bc50-bd07c8be431f | Oracle ID: f88e7647-49dc-4d55-99f6-2f153854e5c0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RITES_OF_REFUSAL,
    oracle_id = "f88e7647-49dc-4d55-99f6-2f153854e5c0",
    scryfall_id = "fa88f595-1b6f-4af0-bc50-bd07c8be431f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Rites of Refusal",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
