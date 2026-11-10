//! Sphere of Grace — {3}{W} — Enchantment
//! Oracle: If a black source would deal damage to you, prevent 2 of that damage.
//! Set: ODY #49 — Odyssey | Scryfall ID: d5b49d82-6d11-4294-a367-0b5b45c05e44 | Oracle ID: e45de1f7-ad45-494d-855e-3e11f109909d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPHERE_OF_GRACE,
    oracle_id = "e45de1f7-ad45-494d-855e-3e11f109909d",
    scryfall_id = "d5b49d82-6d11-4294-a367-0b5b45c05e44",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sphere of Grace",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
