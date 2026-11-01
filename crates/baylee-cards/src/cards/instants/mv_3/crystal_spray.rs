//! Crystal Spray — {2}{U} — Instant
//! Oracle: Change the text of target spell or permanent by replacing all instances of one color word with another or one basic land type with another until end of turn.
//! Oracle: Draw a card.
//! Set: INV #50 — Invasion | Scryfall ID: 8798a4f1-34bb-449d-a8cc-faf8bda8e0ab | Oracle ID: fe371e3a-976f-41d8-ad58-4b3b61415d71
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CRYSTAL_SPRAY,
    oracle_id = "fe371e3a-976f-41d8-ad58-4b3b61415d71",
    scryfall_id = "8798a4f1-34bb-449d-a8cc-faf8bda8e0ab",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Crystal Spray",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
