//! Spawning Pool — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {1}{B}: This land becomes a 1/1 black Skeleton creature with "{B}: Regenerate this creature" until end of turn. It's still a land. (If it regenerates, the next time it would be destroyed this turn, instead tap it, remove it from combat, and heal all damage on it.)
//! Set: 10E #358 — Tenth Edition | Scryfall ID: 907b49ff-2020-4203-b93e-4b3306afc337 | Oracle ID: f3bf22cf-0a6f-4fb6-ba82-63ce290308d6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1071,
    oracle_id: "f3bf22cf-0a6f-4fb6-ba82-63ce290308d6",
    scryfall_id: "907b49ff-2020-4203-b93e-4b3306afc337",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Spawning Pool",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
