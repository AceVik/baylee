//! Abundance — {2}{G}{G} — Enchantment
//! Oracle: If you would draw a card, you may instead choose land or nonland and reveal cards from the top of your library until you reveal a card of the chosen kind. Put that card into your hand and put all other cards revealed this way on the bottom of your library in any order.
//! Set: CMM #884 — Commander Masters | Scryfall ID: 9bc359b0-8886-415b-a497-e98a2241084e | Oracle ID: eccc9a54-2b56-4a02-9926-258d5b2e25fb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ABUNDANCE,
    oracle_id = "eccc9a54-2b56-4a02-9926-258d5b2e25fb",
    scryfall_id = "9bc359b0-8886-415b-a497-e98a2241084e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Abundance",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
