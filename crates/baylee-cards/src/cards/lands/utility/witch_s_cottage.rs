//! Witch's Cottage — (no cost) — Land — Swamp
//! Oracle: ({T}: Add {B}.)
//! Oracle: This land enters tapped unless you control three or more other Swamps.
//! Oracle: When this land enters untapped, you may put target creature card from your graveyard on top of your library.
//! Set: ELD #249 — Throne of Eldraine | Scryfall ID: b87891cd-b457-4dff-8d18-a7eaf6748fc6 | Oracle ID: 6c8f276e-4e7b-4974-ab02-9356cc0ffb2b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1326,
    oracle_id: "6c8f276e-4e7b-4974-ab02-9356cc0ffb2b",
    scryfall_id: "b87891cd-b457-4dff-8d18-a7eaf6748fc6",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Witch's Cottage",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::SWAMP],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
