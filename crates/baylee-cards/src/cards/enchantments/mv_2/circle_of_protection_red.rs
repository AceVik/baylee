//! Circle of Protection: Red — {1}{W} — Enchantment
//! Oracle: {1}: The next time a red source of your choice would deal damage to you this turn, prevent that damage.
//! Set: 9ED #11 — Ninth Edition | Scryfall ID: d7920b6d-ff71-4802-9589-e1df0c58b9ff | Oracle ID: df2738fe-9cd1-4347-8808-105fcfde1190
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CIRCLE_OF_PROTECTION_RED,
    oracle_id = "df2738fe-9cd1-4347-8808-105fcfde1190",
    scryfall_id = "d7920b6d-ff71-4802-9589-e1df0c58b9ff",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Circle of Protection: Red",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
