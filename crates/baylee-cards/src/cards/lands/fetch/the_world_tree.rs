//! The World Tree — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: As long as you control six or more lands, lands you control have "{T}: Add one mana of any color."
//! Oracle: {W}{W}{U}{U}{B}{B}{R}{R}{G}{G}, {T}, Sacrifice this land: Search your library for any number of God cards, put them onto the battlefield, then shuffle.
//! Set: KHM #275 — Kaldheim | Scryfall ID: a70cb6d9-3955-4064-917b-11dec26440c5 | Oracle ID: 3437d504-bf62-4c27-b15f-f6330182ff7e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1183,
    oracle_id: "3437d504-bf62-4c27-b15f-f6330182ff7e",
    scryfall_id: "a70cb6d9-3955-4064-917b-11dec26440c5",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Green, Color::Red, Color::Blue, Color::White]),
    faces: &[
    face! {
        name: "The World Tree",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
