//! Patriarch's Desire — {3}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +2/-2.
//! Oracle: Threshold — Enchanted creature gets an additional +2/-2 as long as there are seven or more cards in your graveyard.
//! Set: ODY #156 — Odyssey | Scryfall ID: 309bc2e7-bece-493d-b7ab-0e08a9f40909 | Oracle ID: 23b518ab-561f-4551-8270-082cde99b993
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PATRIARCH_S_DESIRE,
    oracle_id = "23b518ab-561f-4551-8270-082cde99b993",
    scryfall_id = "309bc2e7-bece-493d-b7ab-0e08a9f40909",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Patriarch's Desire",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
