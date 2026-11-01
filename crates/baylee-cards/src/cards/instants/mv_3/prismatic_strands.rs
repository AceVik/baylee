//! Prismatic Strands — {2}{W} — Instant
//! Oracle: Prevent all damage that sources of the color of your choice would deal this turn.
//! Oracle: Flashback—Tap an untapped white creature you control. (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: C19 #69 — Commander 2019 | Scryfall ID: efd85985-abc1-430a-9210-63109c90a82d | Oracle ID: 8e414ad6-ba19-44a1-a291-2e420734a6dd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PRISMATIC_STRANDS,
    oracle_id = "8e414ad6-ba19-44a1-a291-2e420734a6dd",
    scryfall_id = "efd85985-abc1-430a-9210-63109c90a82d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Prismatic Strands",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
