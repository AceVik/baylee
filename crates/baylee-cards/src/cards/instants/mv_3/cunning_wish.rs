//! Cunning Wish — {2}{U} — Instant
//! Oracle: You may reveal an instant card you own from outside the game and put it into your hand. Exile Cunning Wish.
//! Set: JUD #37 — Judgment | Scryfall ID: ca097675-5e82-493d-beab-9fc11efd7492 | Oracle ID: bdf80a55-b524-4d17-9be8-a0005403a0e7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CUNNING_WISH,
    oracle_id = "bdf80a55-b524-4d17-9be8-a0005403a0e7",
    scryfall_id = "ca097675-5e82-493d-beab-9fc11efd7492",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Cunning Wish",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
