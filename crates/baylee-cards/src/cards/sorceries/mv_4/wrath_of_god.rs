//! Wrath of God — {2}{W}{W} — Sorcery
//! Oracle: Destroy all creatures. They can't be regenerated.
//! Set: CMM #70 — Commander Masters | Scryfall ID: 537d2b05-3f52-45d6-8fe3-26282085d0c6 | Oracle ID: 34515b16-c9a4-4f98-8c77-416a7a523407
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WRATH_OF_GOD,
    oracle_id = "34515b16-c9a4-4f98-8c77-416a7a523407",
    scryfall_id = "537d2b05-3f52-45d6-8fe3-26282085d0c6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Wrath of God",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
