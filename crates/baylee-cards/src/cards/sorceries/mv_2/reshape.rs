//! Reshape — {X}{U}{U} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice an artifact.
//! Oracle: Search your library for an artifact card with mana value X or less, put it onto the battlefield, then shuffle.
//! Set: 2XM #64 — Double Masters | Scryfall ID: 8f8a8f14-bced-4388-b263-5e70431b191f | Oracle ID: 42a3855d-25ab-45b3-9e5d-9a0f3da35a05
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RESHAPE,
    oracle_id = "42a3855d-25ab-45b3-9e5d-9a0f3da35a05",
    scryfall_id = "8f8a8f14-bced-4388-b263-5e70431b191f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Reshape",
        mana_cost = mana!("{X}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
