//! Gamble — {R} — Sorcery
//! Oracle: Search your library for a card, put that card into your hand, discard a card at random, then shuffle.
//! Set: DMR #121 — Dominaria Remastered | Scryfall ID: 8e37fae5-ddd0-4e16-8581-71579f89d9c5 | Oracle ID: a54f0869-94c8-42af-9080-166efb9486a4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GAMBLE,
    oracle_id = "a54f0869-94c8-42af-9080-166efb9486a4",
    scryfall_id = "8e37fae5-ddd0-4e16-8581-71579f89d9c5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Gamble",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
