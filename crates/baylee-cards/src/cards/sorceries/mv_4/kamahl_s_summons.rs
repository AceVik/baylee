//! Kamahl's Summons — {3}{G} — Sorcery
//! Oracle: Each player may reveal any number of creature cards from their hand. Then each player creates a 2/2 green Bear creature token for each card they revealed this way.
//! Set: ONS #269 — Onslaught | Scryfall ID: 0edc37c6-b6a8-424f-95dd-928d03c28542 | Oracle ID: 8b6ef0a2-cd90-44c8-8f11-e1983f2b1dfd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KAMAHL_S_SUMMONS,
    oracle_id = "8b6ef0a2-cd90-44c8-8f11-e1983f2b1dfd",
    scryfall_id = "0edc37c6-b6a8-424f-95dd-928d03c28542",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Kamahl's Summons",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
