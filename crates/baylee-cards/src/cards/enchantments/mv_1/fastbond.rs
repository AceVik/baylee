//! Fastbond — {G} — Enchantment
//! Oracle: You may play any number of lands on each of your turns.
//! Oracle: Whenever you play a land, if it wasn't the first land you played this turn, this enchantment deals 1 damage to you.
//! Set: VMA #209 — Vintage Masters | Scryfall ID: daf43523-558c-4701-9fa3-5d1ceb82a006 | Oracle ID: e27193b7-1a47-4555-865d-b1fd4c6d597f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FASTBOND,
    oracle_id = "e27193b7-1a47-4555-865d-b1fd4c6d597f",
    scryfall_id = "daf43523-558c-4701-9fa3-5d1ceb82a006",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Fastbond",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
