//! Remembrance — {3}{W} — Enchantment
//! Oracle: Whenever a nontoken creature you control dies, you may search your library for a card with the same name as that creature, reveal it, put it into your hand, then shuffle.
//! Set: USG #34 — Urza's Saga | Scryfall ID: 556d334e-3ac9-45b0-98d2-aff49ada0f75 | Oracle ID: 02d0b0fd-ff73-4e26-be44-d85c743d51e4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REMEMBRANCE,
    oracle_id = "02d0b0fd-ff73-4e26-be44-d85c743d51e4",
    scryfall_id = "556d334e-3ac9-45b0-98d2-aff49ada0f75",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Remembrance",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
