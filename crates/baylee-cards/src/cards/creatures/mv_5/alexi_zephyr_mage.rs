//! Alexi, Zephyr Mage — {3}{U}{U} — Legendary Creature — Human Spellshaper
//! Oracle: {X}{U}, {T}, Discard two cards: Return X target creatures to their owners' hands.
//! Set: PCY #28 — Prophecy | Scryfall ID: 6f8fc0b0-4a23-47ed-b61b-a4505fcfc5d2 | Oracle ID: 3f60de36-ed63-4d08-a012-fc16e91da46d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ALEXI_ZEPHYR_MAGE,
    oracle_id = "3f60de36-ed63-4d08-a012-fc16e91da46d",
    scryfall_id = "6f8fc0b0-4a23-47ed-b61b-a4505fcfc5d2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Alexi, Zephyr Mage",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SPELLSHAPER],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
