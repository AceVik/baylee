//! Planeswalker's Mirth — {2}{W} — Enchantment
//! Oracle: {3}{W}: Target opponent reveals a card at random from their hand. You gain life equal to that card's mana value.
//! Set: PLS #12 — Planeshift | Scryfall ID: 0205d094-c846-4aa0-ade8-2a52c57b11da | Oracle ID: cebc1e6d-ab14-4e54-9906-e8a7cf4c2043
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLANESWALKER_S_MIRTH,
    oracle_id = "cebc1e6d-ab14-4e54-9906-e8a7cf4c2043",
    scryfall_id = "0205d094-c846-4aa0-ade8-2a52c57b11da",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Planeswalker's Mirth",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
