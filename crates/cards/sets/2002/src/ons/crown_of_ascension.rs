//! Crown of Ascension — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has flying.
//! Oracle: Sacrifice this Aura: Enchanted creature and other creatures that share a creature type with it gain flying until end of turn.
//! Set: ONS #78 — Onslaught | Scryfall ID: 2fe86733-7851-4c2a-8d94-dba6f071b94d | Oracle ID: 32ddb0a3-2848-4939-87ad-ebcc5547fd60
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CROWN_OF_ASCENSION,
    oracle_id = "32ddb0a3-2848-4939-87ad-ebcc5547fd60",
    scryfall_id = "2fe86733-7851-4c2a-8d94-dba6f071b94d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Crown of Ascension",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
