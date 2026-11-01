//! Outbreak — {3}{B} — Sorcery
//! Oracle: You may discard a Swamp card rather than pay this spell's mana cost.
//! Oracle: Choose a creature type. All creatures of that type get -1/-1 until end of turn.
//! Set: PCY #71 — Prophecy | Scryfall ID: f43c30d9-23a5-4872-925d-3427f5f57995 | Oracle ID: 05079cee-b190-42d7-b59f-130cb545cab7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OUTBREAK,
    oracle_id = "05079cee-b190-42d7-b59f-130cb545cab7",
    scryfall_id = "f43c30d9-23a5-4872-925d-3427f5f57995",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Outbreak",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
