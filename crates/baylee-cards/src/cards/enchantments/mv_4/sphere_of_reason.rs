//! Sphere of Reason — {3}{W} — Enchantment
//! Oracle: If a blue source would deal damage to you, prevent 2 of that damage.
//! Set: ODY #51 — Odyssey | Scryfall ID: 76ef2016-b3c8-4615-a056-3000bcccb68e | Oracle ID: de85c961-622d-4d70-8cd7-f722c1ac5ed1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPHERE_OF_REASON,
    oracle_id = "de85c961-622d-4d70-8cd7-f722c1ac5ed1",
    scryfall_id = "76ef2016-b3c8-4615-a056-3000bcccb68e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sphere of Reason",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
