//! Ipnu Rivulet — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Pay 1 life: Add {U}.
//! Oracle: {1}{U}, {T}, Sacrifice a Desert: Target player mills four cards. (They put the top four cards of their library into their graveyard.)
//! Set: HOU #180 — Hour of Devastation | Scryfall ID: 203011ef-3737-4fd1-bd23-0e531b5a7c32 | Oracle ID: c17d799f-adc9-4c41-87cf-b243b5ea3be1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 656,
    oracle_id: "c17d799f-adc9-4c41-87cf-b243b5ea3be1",
    scryfall_id: "203011ef-3737-4fd1-bd23-0e531b5a7c32",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
    face! {
        name: "Ipnu Rivulet",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
