//! Golden Wish — {3}{W}{W} — Sorcery
//! Oracle: You may reveal an artifact or enchantment card you own from outside the game and put it into your hand. Exile Golden Wish.
//! Set: JUD #12 — Judgment | Scryfall ID: dc409ded-41f3-4f14-8199-72a9fe98bac0 | Oracle ID: e5f25cbd-39e4-4806-8dba-9a552356696d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GOLDEN_WISH,
    oracle_id = "e5f25cbd-39e4-4806-8dba-9a552356696d",
    scryfall_id = "dc409ded-41f3-4f14-8199-72a9fe98bac0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Golden Wish",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
