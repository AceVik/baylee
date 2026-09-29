//! Terror — {1}{B} — Instant
//! Oracle: Destroy target nonartifact, nonblack creature. It can't be regenerated.
//! Set: DMR #103 — Dominaria Remastered | Scryfall ID: a3870eba-4e8f-414d-817c-14ec97f6a93c | Oracle ID: b81f041d-98db-4408-9472-c483e4a502bc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TERROR,
    oracle_id = "b81f041d-98db-4408-9472-c483e4a502bc",
    scryfall_id = "a3870eba-4e8f-414d-817c-14ec97f6a93c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Terror",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
