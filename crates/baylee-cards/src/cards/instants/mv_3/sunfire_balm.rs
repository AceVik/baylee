//! Sunfire Balm — {2}{W} — Instant
//! Oracle: Prevent the next 4 damage that would be dealt to any target this turn.
//! Oracle: Cycling {1}{W} ({1}{W}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, you may prevent the next 1 damage that would be dealt to any target this turn.
//! Set: ONS #56 — Onslaught | Scryfall ID: 0d563ebb-ecd1-406c-9d69-c101acdeced7 | Oracle ID: 61d2f269-8674-4429-a07c-59972afd9ede
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SUNFIRE_BALM,
    oracle_id = "61d2f269-8674-4429-a07c-59972afd9ede",
    scryfall_id = "0d563ebb-ecd1-406c-9d69-c101acdeced7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sunfire Balm",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
