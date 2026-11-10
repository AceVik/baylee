//! Restrain — {2}{W} — Instant
//! Oracle: Prevent all combat damage that would be dealt by target attacking creature this turn.
//! Oracle: Draw a card.
//! Set: INV #30 — Invasion | Scryfall ID: f6b5c765-619c-4db9-b509-91892fb65e8f | Oracle ID: 5aa66cb0-86b9-4e85-9a17-df78416d682d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RESTRAIN,
    oracle_id = "5aa66cb0-86b9-4e85-9a17-df78416d682d",
    scryfall_id = "f6b5c765-619c-4db9-b509-91892fb65e8f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Restrain",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
