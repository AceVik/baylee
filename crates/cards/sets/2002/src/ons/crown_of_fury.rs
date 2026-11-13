//! Crown of Fury — {1}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+0 and has first strike.
//! Oracle: Sacrifice this Aura: Enchanted creature and other creatures that share a creature type with it get +1/+0 and gain first strike until end of turn.
//! Set: ONS #196 — Onslaught | Scryfall ID: 6caae974-f531-469d-8c6a-2077c4f3294a | Oracle ID: 8ce4a4c0-3350-43ef-95d9-2b2f76ca61d9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CROWN_OF_FURY,
    oracle_id = "8ce4a4c0-3350-43ef-95d9-2b2f76ca61d9",
    scryfall_id = "6caae974-f531-469d-8c6a-2077c4f3294a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Crown of Fury",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
