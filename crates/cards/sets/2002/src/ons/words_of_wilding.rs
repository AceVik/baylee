//! Words of Wilding — {2}{G} — Enchantment
//! Oracle: {1}: The next time you would draw a card this turn, create a 2/2 green Bear creature token instead.
//! Set: ONS #305 — Onslaught | Scryfall ID: fdb9565f-5b09-4127-b169-3146079dab84 | Oracle ID: 888582df-4fe2-4d05-9b62-10c90aaa76f2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WORDS_OF_WILDING,
    oracle_id = "888582df-4fe2-4d05-9b62-10c90aaa76f2",
    scryfall_id = "fdb9565f-5b09-4127-b169-3146079dab84",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Words of Wilding",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
