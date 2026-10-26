//! Traveler's Cloak — {2}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: As this Aura enters, choose a land type.
//! Oracle: When this Aura enters, draw a card.
//! Oracle: Enchanted creature has landwalk of the chosen type. (It can't be blocked as long as defending player controls a land of that type.)
//! Set: CNS #109 — Conspiracy | Scryfall ID: c29e4a61-8f84-4c19-909d-6aebb3b9ab9b | Oracle ID: 5241182d-e2ca-4b31-809f-fbff2822dbf6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TRAVELER_S_CLOAK,
    oracle_id = "5241182d-e2ca-4b31-809f-fbff2822dbf6",
    scryfall_id = "c29e4a61-8f84-4c19-909d-6aebb3b9ab9b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Traveler's Cloak",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
