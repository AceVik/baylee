//! Feedback — {2}{U} — Enchantment — Aura
//! Oracle: Enchant enchantment
//! Oracle: At the beginning of the upkeep of enchanted enchantment's controller, this Aura deals 1 damage to that player.
//! Set: 5ED #85 — Fifth Edition | Scryfall ID: 1d452de7-3f44-4594-bb24-2178812da9d6 | Oracle ID: 2ff92886-5c17-47a4-a02b-a97432d9203e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FEEDBACK,
    oracle_id = "2ff92886-5c17-47a4-a02b-a97432d9203e",
    scryfall_id = "1d452de7-3f44-4594-bb24-2178812da9d6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Feedback",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
