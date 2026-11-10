//! Winnow — {1}{W} — Instant
//! Oracle: Destroy target nonland permanent if another permanent with the same name is on the battlefield.
//! Oracle: Draw a card.
//! Set: INV #45 — Invasion | Scryfall ID: d61748dd-4010-47da-8717-ca0147877057 | Oracle ID: 1b2ad355-3f1e-4c8b-93ee-848934646b23
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WINNOW,
    oracle_id = "1b2ad355-3f1e-4c8b-93ee-848934646b23",
    scryfall_id = "d61748dd-4010-47da-8717-ca0147877057",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Winnow",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
