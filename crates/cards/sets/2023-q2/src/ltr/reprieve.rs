//! Reprieve — {1}{W} — Instant
//! Oracle: Return target spell to its owner's hand.
//! Oracle: Draw a card.
//! Set: LTR #26 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: 1bd3fa8a-6c50-4f7f-9ae3-0810eec5e3db | Oracle ID: f0449cbd-855c-4c20-a1f5-f76a395d8d39
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REPRIEVE,
    oracle_id = "f0449cbd-855c-4c20-a1f5-f76a395d8d39",
    scryfall_id = "1bd3fa8a-6c50-4f7f-9ae3-0810eec5e3db",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Reprieve",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
