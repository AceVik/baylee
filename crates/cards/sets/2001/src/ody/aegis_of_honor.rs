//! Aegis of Honor — {W} — Enchantment
//! Oracle: {1}: The next time an instant or sorcery spell would deal damage to you this turn, that spell deals that damage to its controller instead.
//! Set: ODY #1 — Odyssey | Scryfall ID: 9561ffdb-f4dd-4b62-a2c2-933498e3a061 | Oracle ID: ea93acc8-0c1f-42a2-bed3-f385d210d58f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AEGIS_OF_HONOR,
    oracle_id = "ea93acc8-0c1f-42a2-bed3-f385d210d58f",
    scryfall_id = "9561ffdb-f4dd-4b62-a2c2-933498e3a061",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Aegis of Honor",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
