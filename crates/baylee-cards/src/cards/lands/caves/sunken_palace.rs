//! Sunken Palace — (no cost) — Land — Cave
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: {1}{U}, {T}, Exile seven cards from your graveyard: Add {U}. When you spend this mana to cast a spell or activate an ability, copy that spell or ability. You may choose new targets for the copy. (Mana abilities can't be copied.)
//! Set: M3C #81 — Modern Horizons 3 Commander | Scryfall ID: e44ee47c-95de-4090-97b6-188585d86b0c | Oracle ID: c098c507-5154-423a-a70b-f6dfd4959cf6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1115,
    oracle_id: "c098c507-5154-423a-a70b-f6dfd4959cf6",
    scryfall_id: "e44ee47c-95de-4090-97b6-188585d86b0c",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
    face! {
        name: "Sunken Palace",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
