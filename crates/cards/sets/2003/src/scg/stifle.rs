//! Stifle — {U} — Instant
//! Oracle: Counter target activated or triggered ability. (Mana abilities can't be targeted.)
//! Set: CNS #108 — Conspiracy | Scryfall ID: 616d1b20-61c1-4d39-a9b5-ad9fd61699e4 | Oracle ID: b3b00911-ece7-4484-bc36-f211ce72b6cc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STIFLE,
    oracle_id = "b3b00911-ece7-4484-bc36-f211ce72b6cc",
    scryfall_id = "616d1b20-61c1-4d39-a9b5-ad9fd61699e4",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Stifle",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
