//! Disorder — {1}{R} — Sorcery
//! Oracle: Disorder deals 2 damage to each white creature and each player who controls a white creature.
//! Set: USG #181 — Urza's Saga | Scryfall ID: 3fa5ec10-dfea-4e6d-8996-553a4a0eb8a4 | Oracle ID: 586da84b-1384-4da2-983d-340c22b73eb8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DISORDER,
    oracle_id = "586da84b-1384-4da2-983d-340c22b73eb8",
    scryfall_id = "3fa5ec10-dfea-4e6d-8996-553a4a0eb8a4",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Disorder",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
