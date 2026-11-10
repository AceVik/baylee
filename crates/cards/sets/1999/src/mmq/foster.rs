//! Foster — {2}{G}{G} — Enchantment
//! Oracle: Whenever a creature you control dies, you may pay {1}. If you do, reveal cards from the top of your library until you reveal a creature card. Put that card into your hand and the rest into your graveyard.
//! Set: C13 #146 — Commander 2013 | Scryfall ID: 00f86a0b-0f46-4ce1-bbeb-7f46881e1627 | Oracle ID: fb431500-152c-4524-b76b-de62922ff57f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FOSTER,
    oracle_id = "fb431500-152c-4524-b76b-de62922ff57f",
    scryfall_id = "00f86a0b-0f46-4ce1-bbeb-7f46881e1627",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Foster",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
