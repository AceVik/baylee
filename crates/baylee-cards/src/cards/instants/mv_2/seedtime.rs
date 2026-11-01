//! Seedtime — {1}{G} — Instant
//! Oracle: Cast this spell only during your turn.
//! Oracle: Take an extra turn after this one if an opponent cast a blue spell this turn.
//! Set: JUD #130 — Judgment | Scryfall ID: 4ffd5c52-f260-400c-b088-8792282509a5 | Oracle ID: 3ba50802-57d0-450e-9e2f-a46a09d2f6bb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEEDTIME,
    oracle_id = "3ba50802-57d0-450e-9e2f-a46a09d2f6bb",
    scryfall_id = "4ffd5c52-f260-400c-b088-8792282509a5",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Seedtime",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
