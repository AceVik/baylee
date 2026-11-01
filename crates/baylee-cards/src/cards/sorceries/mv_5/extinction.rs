//! Extinction — {4}{B} — Sorcery
//! Oracle: Destroy all creatures of the creature type of your choice.
//! Set: TMP #135 — Tempest | Scryfall ID: a233a244-7f84-4525-b0ce-e10db0a95385 | Oracle ID: f3f01fe8-0a2c-4afa-94d2-268d27c0d252
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EXTINCTION,
    oracle_id = "f3f01fe8-0a2c-4afa-94d2-268d27c0d252",
    scryfall_id = "a233a244-7f84-4525-b0ce-e10db0a95385",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Extinction",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
