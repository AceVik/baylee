//! Cephalid Shrine — {1}{U}{U} — Enchantment
//! Oracle: Whenever a player casts a spell, counter that spell unless that player pays {X}, where X is the number of cards in all graveyards with the same name as the spell.
//! Set: ODY #75 — Odyssey | Scryfall ID: 137e0ac1-3bf2-4583-b44f-c9fe8d7e8882 | Oracle ID: 95f05f77-bdc3-4799-910b-5ca7ffadffe3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CEPHALID_SHRINE,
    oracle_id = "95f05f77-bdc3-4799-910b-5ca7ffadffe3",
    scryfall_id = "137e0ac1-3bf2-4583-b44f-c9fe8d7e8882",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Cephalid Shrine",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
