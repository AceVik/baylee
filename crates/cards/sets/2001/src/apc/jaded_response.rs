//! Jaded Response — {1}{U} — Instant
//! Oracle: Counter target spell if it shares a color with a creature you control.
//! Set: APC #26 — Apocalypse | Scryfall ID: 6a9ab1f0-4e75-4165-85bc-6f838c221d6a | Oracle ID: 23917517-7561-45b3-b156-77cd2f340466
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JADED_RESPONSE,
    oracle_id = "23917517-7561-45b3-b156-77cd2f340466",
    scryfall_id = "6a9ab1f0-4e75-4165-85bc-6f838c221d6a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Jaded Response",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
