//! Landslide — {R} — Sorcery
//! Oracle: Sacrifice any number of Mountains. Landslide deals that much damage to target player or planeswalker.
//! Set: UDS #92 — Urza's Destiny | Scryfall ID: c0ddc0dc-8783-4659-bbbd-db6698843b47 | Oracle ID: f15761cf-2ac5-4458-957c-15badb1dda50
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LANDSLIDE,
    oracle_id = "f15761cf-2ac5-4458-957c-15badb1dda50",
    scryfall_id = "c0ddc0dc-8783-4659-bbbd-db6698843b47",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Landslide",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
