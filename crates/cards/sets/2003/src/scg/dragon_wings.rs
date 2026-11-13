//! Dragon Wings — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has flying.
//! Oracle: Cycling {1}{U} ({1}{U}, Discard this card: Draw a card.)
//! Oracle: When a creature with mana value 6 or greater enters, you may return this card from your graveyard to the battlefield attached to that creature.
//! Set: SCG #34 — Scourge | Scryfall ID: 7674ab4d-9bc0-45c3-88e1-3fd2c947cfaa | Oracle ID: 85271333-af96-447a-8a16-9fadf0eaedb2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRAGON_WINGS,
    oracle_id = "85271333-af96-447a-8a16-9fadf0eaedb2",
    scryfall_id = "7674ab4d-9bc0-45c3-88e1-3fd2c947cfaa",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Dragon Wings",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
