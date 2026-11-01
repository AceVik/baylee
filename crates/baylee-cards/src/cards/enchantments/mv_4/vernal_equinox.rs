//! Vernal Equinox — {3}{G} — Enchantment
//! Oracle: Any player may cast creature and enchantment spells as though they had flash.
//! Set: MMQ #283 — Mercadian Masques | Scryfall ID: 3bed69d2-f5fb-4173-b939-5abdb48b82b4 | Oracle ID: 2241ef7d-3e9d-44ee-88ac-e8ad6fb2444e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VERNAL_EQUINOX,
    oracle_id = "2241ef7d-3e9d-44ee-88ac-e8ad6fb2444e",
    scryfall_id = "3bed69d2-f5fb-4173-b939-5abdb48b82b4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Vernal Equinox",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
