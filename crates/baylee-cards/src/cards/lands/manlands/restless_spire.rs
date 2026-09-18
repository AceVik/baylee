//! Restless Spire — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U} or {R}.
//! Oracle: {U}{R}: Until end of turn, this land becomes a 2/1 blue and red Elemental creature with "During your turn, this creature has first strike." It's still a land.
//! Oracle: Whenever this land attacks, scry 1.
//! Set: FRC #82 — Reality Fracture Commander | Scryfall ID: 30ecfd44-8dd4-4ae3-9076-14799f8a9a14 | Oracle ID: 0ca4e80e-c19c-4b74-b531-c5a4dc5a8ba9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RESTLESS_SPIRE,
    oracle_id = "0ca4e80e-c19c-4b74-b531-c5a4dc5a8ba9",
    scryfall_id = "30ecfd44-8dd4-4ae3-9076-14799f8a9a14",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[face!(name = "Restless Spire", types = TypeSet::LAND,),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
