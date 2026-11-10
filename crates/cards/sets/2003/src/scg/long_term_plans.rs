//! Long-Term Plans — {2}{U} — Instant
//! Oracle: Search your library for a card, then shuffle and put that card third from the top.
//! Set: SCG #38 — Scourge | Scryfall ID: 7e0422d9-9694-45b6-9c2b-2ca31198cebf | Oracle ID: d1dfb359-d8f9-491e-88f7-95193b220200
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LONG_TERM_PLANS,
    oracle_id = "d1dfb359-d8f9-491e-88f7-95193b220200",
    scryfall_id = "7e0422d9-9694-45b6-9c2b-2ca31198cebf",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Long-Term Plans",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
