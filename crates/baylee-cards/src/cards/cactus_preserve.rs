//! Cactus Preserve — (no cost) — Land — Desert
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add one mana of any type that a land you control could produce.
//! Oracle: {3}: Until end of turn, this land becomes an X/X green Plant creature with reach, where X is the greatest mana value among your commanders. It's still a land.
//! Set: OTC #40 — Outlaws of Thunder Junction Commander | Scryfall ID: ad9d426f-5870-42bb-a589-9218f7e35d62 | Oracle ID: 8da29533-f389-4bc2-ab9b-b469f893a362
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 323,
    oracle_id: "8da29533-f389-4bc2-ab9b-b469f893a362",
    scryfall_id: "ad9d426f-5870-42bb-a589-9218f7e35d62",
    faces: &[
    face! {
        name: "Cactus Preserve",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
