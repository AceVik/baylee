//! Desperate Gambit — {R} — Instant
//! Oracle: Choose a source you control and flip a coin. If you win the flip, the next time that source would deal damage this turn, it deals double that damage instead. If you lose the flip, the next time it would deal damage this turn, prevent that damage.
//! Set: WTH #96 — Weatherlight | Scryfall ID: f4245160-274e-4c39-9bcd-c64e9a44dfdb | Oracle ID: d8328d27-e27f-4c84-8ab5-cabb80a34f54
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DESPERATE_GAMBIT,
    oracle_id = "d8328d27-e27f-4c84-8ab5-cabb80a34f54",
    scryfall_id = "f4245160-274e-4c39-9bcd-c64e9a44dfdb",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Desperate Gambit",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
