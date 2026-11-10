//! Pedantic Learning — {U}{U} — Enchantment
//! Oracle: Whenever a land card is put into your graveyard from your library, you may pay {1}. If you do, draw a card.
//! Set: ODY #90 — Odyssey | Scryfall ID: 67445332-0b56-4fec-8dcd-bb9a87194db5 | Oracle ID: 194c0ace-4c5d-4698-a924-901bdef99a5f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PEDANTIC_LEARNING,
    oracle_id = "194c0ace-4c5d-4698-a924-901bdef99a5f",
    scryfall_id = "67445332-0b56-4fec-8dcd-bb9a87194db5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Pedantic Learning",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
