//! Psychic Trance — {2}{U}{U} — Instant
//! Oracle: Until end of turn, Wizards you control gain "{T}: Counter target spell."
//! Set: ONS #102 — Onslaught | Scryfall ID: d5e55695-16cc-4373-8078-959f1ded4c6d | Oracle ID: 5870175d-83df-43af-b199-fcd1e60b824e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PSYCHIC_TRANCE,
    oracle_id = "5870175d-83df-43af-b199-fcd1e60b824e",
    scryfall_id = "d5e55695-16cc-4373-8078-959f1ded4c6d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Psychic Trance",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
