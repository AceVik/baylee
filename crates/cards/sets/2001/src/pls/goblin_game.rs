//! Goblin Game — {5}{R}{R} — Sorcery
//! Oracle: Each player hides at least one item, then all players reveal them simultaneously. Each player loses life equal to the number of items they revealed. The player who revealed the fewest items then loses half their life, rounded up. If two or more players are tied for fewest, each loses half their life, rounded up.
//! Set: PLS #61 — Planeshift | Scryfall ID: cbe6e7e5-ffea-4c6c-8a42-28e695029f24 | Oracle ID: 4a10c945-ad8b-47b6-8cbd-7b4e2612a9b0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GOBLIN_GAME,
    oracle_id = "4a10c945-ad8b-47b6-8cbd-7b4e2612a9b0",
    scryfall_id = "cbe6e7e5-ffea-4c6c-8a42-28e695029f24",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Goblin Game",
        mana_cost = mana!("{5}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
