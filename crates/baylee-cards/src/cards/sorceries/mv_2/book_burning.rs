//! Book Burning — {1}{R} — Sorcery
//! Oracle: Any player may have Book Burning deal 6 damage to them. If no one does, target player mills six cards.
//! Set: JUD #80 — Judgment | Scryfall ID: bead678c-7b6a-4668-9919-623312e08a65 | Oracle ID: 644a8946-4890-4c2b-978f-02eedc95b5ad
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BOOK_BURNING,
    oracle_id = "644a8946-4890-4c2b-978f-02eedc95b5ad",
    scryfall_id = "bead678c-7b6a-4668-9919-623312e08a65",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Book Burning",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
