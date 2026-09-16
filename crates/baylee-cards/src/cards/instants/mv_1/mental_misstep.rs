//! Mental Misstep — {U/P} — Instant
//! Oracle: ({U/P} can be paid with either {U} or 2 life.)
//! Oracle: Counter target spell with mana value 1.
//! Set: NPH #38 — New Phyrexia | Scryfall ID: 61e9c6df-1c84-4eab-9076-a4feb6347c10 | Oracle ID: 1a0770e6-b093-4439-baff-6889a50ba12e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MENTAL_MISSTEP,
    oracle_id = "1a0770e6-b093-4439-baff-6889a50ba12e",
    scryfall_id = "61e9c6df-1c84-4eab-9076-a4feb6347c10",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mental Misstep",
        mana_cost = mana!("{U/P}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
