//! Karma — {2}{W}{W} — Enchantment
//! Oracle: At the beginning of each player's upkeep, this enchantment deals damage to that player equal to the number of Swamps they control.
//! Set: 8ED #28 — Eighth Edition | Scryfall ID: cf9df1cc-9de3-42a6-9c28-f2d8db75322a | Oracle ID: fac4da47-f0b2-4b57-9703-e0ed100d3499
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KARMA,
    oracle_id = "fac4da47-f0b2-4b57-9703-e0ed100d3499",
    scryfall_id = "cf9df1cc-9de3-42a6-9c28-f2d8db75322a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Karma",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
