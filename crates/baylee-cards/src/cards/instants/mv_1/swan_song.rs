//! Swan Song — {U} — Instant
//! Oracle: Counter target enchantment, instant, or sorcery spell. Its controller creates a 2/2 blue Bird creature token with flying.
//! Set: EOC #46 — Edge of Eternities Commander | Scryfall ID: 83d0b761-d694-4232-9f40-5bd8c82a05f1 | Oracle ID: 8ddfc283-c9b4-41a5-af88-cf0068e986cc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SWAN_SONG,
    oracle_id = "8ddfc283-c9b4-41a5-af88-cf0068e986cc",
    scryfall_id = "83d0b761-d694-4232-9f40-5bd8c82a05f1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Swan Song",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
