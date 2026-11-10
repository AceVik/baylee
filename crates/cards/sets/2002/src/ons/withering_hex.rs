//! Withering Hex — {B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever a player cycles a card, put a plague counter on this Aura.
//! Oracle: Enchanted creature gets -1/-1 for each plague counter on this Aura.
//! Set: ONS #181 — Onslaught | Scryfall ID: 9ce4be1e-97dd-45ec-89e5-2fb56145c098 | Oracle ID: 96e41850-04a9-47bf-98a7-4238b25b1824
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WITHERING_HEX,
    oracle_id = "96e41850-04a9-47bf-98a7-4238b25b1824",
    scryfall_id = "9ce4be1e-97dd-45ec-89e5-2fb56145c098",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Withering Hex",
        mana_cost = mana!("{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
