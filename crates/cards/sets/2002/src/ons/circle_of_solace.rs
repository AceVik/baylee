//! Circle of Solace — {3}{W} — Enchantment
//! Oracle: As this enchantment enters, choose a creature type.
//! Oracle: {1}{W}: The next time a creature of the chosen type would deal damage to you this turn, prevent that damage.
//! Set: ONS #13 — Onslaught | Scryfall ID: 07f567dc-8a60-40e1-b947-199872d8df08 | Oracle ID: 14ebedd1-7a2d-413b-9b76-8610511292a4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CIRCLE_OF_SOLACE,
    oracle_id = "14ebedd1-7a2d-413b-9b76-8610511292a4",
    scryfall_id = "07f567dc-8a60-40e1-b947-199872d8df08",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Circle of Solace",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
