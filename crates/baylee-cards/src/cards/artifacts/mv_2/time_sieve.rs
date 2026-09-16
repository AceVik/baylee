//! Time Sieve — {U}{B} — Artifact
//! Oracle: {T}, Sacrifice five artifacts: Take an extra turn after this one.
//! Set: 2XM #223 — Double Masters | Scryfall ID: c2e8b424-0cec-490e-a571-bd051f952adf | Oracle ID: 3da5977a-36d4-4f32-ab9b-8b93809d818d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TIME_SIEVE,
    oracle_id = "3da5977a-36d4-4f32-ab9b-8b93809d818d",
    scryfall_id = "c2e8b424-0cec-490e-a571-bd051f952adf",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Time Sieve",
        mana_cost = mana!("{U}{B}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
