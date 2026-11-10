//! Sway of Illusion — {1}{U} — Instant
//! Oracle: Any number of target creatures become the color of your choice until end of turn.
//! Oracle: Draw a card.
//! Set: INV #77 — Invasion | Scryfall ID: ff65e386-9aec-4deb-a4ec-d9a97bd87645 | Oracle ID: 9a7e2298-8855-43a7-8cb1-4b31e4058c3f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SWAY_OF_ILLUSION,
    oracle_id = "9a7e2298-8855-43a7-8cb1-4b31e4058c3f",
    scryfall_id = "ff65e386-9aec-4deb-a4ec-d9a97bd87645",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Sway of Illusion",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
