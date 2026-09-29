//! Power Leak — {1}{U} — Enchantment — Aura
//! Oracle: Enchant enchantment
//! Oracle: At the beginning of the upkeep of enchanted enchantment's controller, that player may pay any amount of mana. This Aura deals 2 damage to that player. Prevent X of that damage, where X is the amount of mana that player paid this way.
//! Set: 4ED #92 — Fourth Edition | Scryfall ID: 99f235d7-8f6b-4d17-bc36-6f2cb6d5deec | Oracle ID: dc2f0000-870b-487f-9623-618fc8eb9765
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::POWER_LEAK,
    oracle_id = "dc2f0000-870b-487f-9623-618fc8eb9765",
    scryfall_id = "99f235d7-8f6b-4d17-bc36-6f2cb6d5deec",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Power Leak",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
