//! Deflecting Swat — {2}{R} — Instant
//! Oracle: If you control a commander, you may cast this spell without paying its mana cost.
//! Oracle: You may choose new targets for target spell or ability.
//! Set: CMM #214 — Commander Masters | Scryfall ID: b4b36435-55b3-4615-8812-af41d4fc64d9 | Oracle ID: ae120613-97d6-4393-b39d-c3e6c076f5d6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEFLECTING_SWAT,
    oracle_id = "ae120613-97d6-4393-b39d-c3e6c076f5d6",
    scryfall_id = "b4b36435-55b3-4615-8812-af41d4fc64d9",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Deflecting Swat",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
