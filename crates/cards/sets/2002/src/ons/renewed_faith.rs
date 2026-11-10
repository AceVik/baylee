//! Renewed Faith — {2}{W} — Instant
//! Oracle: You gain 6 life.
//! Oracle: Cycling {1}{W} ({1}{W}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, you may gain 2 life.
//! Set: DMR #23 — Dominaria Remastered | Scryfall ID: a7d66dd3-330f-4b12-81f6-52d7a31013a9 | Oracle ID: aa4ecefa-808e-4706-af16-4ea5af9743fc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RENEWED_FAITH,
    oracle_id = "aa4ecefa-808e-4706-af16-4ea5af9743fc",
    scryfall_id = "a7d66dd3-330f-4b12-81f6-52d7a31013a9",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Renewed Faith",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
