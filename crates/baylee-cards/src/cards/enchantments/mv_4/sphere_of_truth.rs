//! Sphere of Truth — {3}{W} — Enchantment
//! Oracle: If a white source would deal damage to you, prevent 2 of that damage.
//! Set: ODY #52 — Odyssey | Scryfall ID: 7c3de9f5-a05c-4341-a2c2-a1a475d400fd | Oracle ID: e6325604-cdce-4872-af03-e436631004de
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPHERE_OF_TRUTH,
    oracle_id = "e6325604-cdce-4872-af03-e436631004de",
    scryfall_id = "7c3de9f5-a05c-4341-a2c2-a1a475d400fd",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sphere of Truth",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
