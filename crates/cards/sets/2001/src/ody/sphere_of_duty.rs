//! Sphere of Duty — {3}{W} — Enchantment
//! Oracle: If a green source would deal damage to you, prevent 2 of that damage.
//! Set: ODY #48 — Odyssey | Scryfall ID: 4bdcd0de-6466-488c-a882-bad80c86504d | Oracle ID: c1612eef-5059-46fd-a5fd-c8ebf6779a6e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPHERE_OF_DUTY,
    oracle_id = "c1612eef-5059-46fd-a5fd-c8ebf6779a6e",
    scryfall_id = "4bdcd0de-6466-488c-a882-bad80c86504d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sphere of Duty",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
