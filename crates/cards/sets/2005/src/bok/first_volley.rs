//! First Volley — {1}{R} — Instant — Arcane
//! Oracle: First Volley deals 1 damage to target creature and 1 damage to that creature's controller.
//! Set: BOK #100 — Betrayers of Kamigawa | Scryfall ID: d6e5e360-ed47-40c1-8ad7-57645c2854ca | Oracle ID: b6a89457-1581-46d4-b75b-c5b8704d9f6f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FIRST_VOLLEY,
    oracle_id = "b6a89457-1581-46d4-b75b-c5b8704d9f6f",
    scryfall_id = "d6e5e360-ed47-40c1-8ad7-57645c2854ca",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "First Volley",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
        subtypes = &[subtypes::spell::ARCANE],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
