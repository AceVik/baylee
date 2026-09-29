//! Burrowing — {R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has mountainwalk. (It can't be blocked as long as defending player controls a Mountain.)
//! Set: 6ED #170 — Classic Sixth Edition | Scryfall ID: c665180c-a69b-446e-9894-3b3be624db7f | Oracle ID: d6b9b88b-e31b-4b88-9d53-3df5687804ba
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BURROWING,
    oracle_id = "d6b9b88b-e31b-4b88-9d53-3df5687804ba",
    scryfall_id = "c665180c-a69b-446e-9894-3b3be624db7f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Burrowing",
        mana_cost = mana!("{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
