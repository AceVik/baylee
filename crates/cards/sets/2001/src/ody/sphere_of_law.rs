//! Sphere of Law — {3}{W} — Enchantment
//! Oracle: If a red source would deal damage to you, prevent 2 of that damage.
//! Set: ODY #50 — Odyssey | Scryfall ID: f3451ddf-efec-474b-884c-64288e5552d2 | Oracle ID: a8da7c57-c60f-42b1-b611-64a0a43295f2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPHERE_OF_LAW,
    oracle_id = "a8da7c57-c60f-42b1-b611-64a0a43295f2",
    scryfall_id = "f3451ddf-efec-474b-884c-64288e5552d2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sphere of Law",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
