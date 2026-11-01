//! Radiate — {3}{R}{R} — Instant
//! Oracle: Choose target instant or sorcery spell that targets only a single permanent or player. Copy that spell for each other permanent or player the spell could target. Each copy targets a different one of those permanents and players.
//! Set: TOR #113 — Torment | Scryfall ID: 2562c25a-999e-4fb5-a595-f376c8abf1ff | Oracle ID: 2e5163bd-9b8a-4d1e-9ba9-8666c7348dfd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RADIATE,
    oracle_id = "2e5163bd-9b8a-4d1e-9ba9-8666c7348dfd",
    scryfall_id = "2562c25a-999e-4fb5-a595-f376c8abf1ff",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Radiate",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
