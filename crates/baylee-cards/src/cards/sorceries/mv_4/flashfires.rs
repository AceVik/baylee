//! Flashfires — {3}{R} — Sorcery
//! Oracle: Destroy all Plains.
//! Set: 9ED #183 — Ninth Edition | Scryfall ID: 277e0846-237e-4117-bdd4-d2e4ae450d43 | Oracle ID: c281f436-8c77-48f7-b31c-d40cd7f9ed6a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FLASHFIRES,
    oracle_id = "c281f436-8c77-48f7-b31c-d40cd7f9ed6a",
    scryfall_id = "277e0846-237e-4117-bdd4-d2e4ae450d43",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Flashfires",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
