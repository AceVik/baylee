//! Misty Rainforest — (no cost) — Land
//! Oracle: {T}, Pay 1 life, Sacrifice this land: Search your library for a Forest or Island card, put it onto the battlefield, then shuffle.
//! Set: MH2 #250 — Modern Horizons 2 | Scryfall ID: 88231c0d-0cc8-44ec-bf95-81d1710ac141 | Oracle ID: 09dd85aa-47bc-4713-a9b9-8b52ff2285ed
// IMPLEMENTED — fetchland (tap + pay life + sacrifice → search Forest/Island
// to the battlefield tapped, shuffle).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SEARCH_FILTER: Filter = Filter::Or(&[
    Filter::HasSubtype(land::FOREST),
    Filter::HasSubtype(land::ISLAND),
]);

card! {
    index: 97,
    oracle_id: "09dd85aa-47bc-4713-a9b9-8b52ff2285ed",
    scryfall_id: "88231c0d-0cc8-44ec-bf95-81d1710ac141",
    faces: &[face! {
        name: "Misty Rainforest",
        types: TypeSet::LAND,
    }],
    coverage: Coverage::Implemented,
    abilities: &[activated!(Cost {
            mana: ManaCost::ZERO,
            parts: &[
                CostPart::TapSelf,
                CostPart::SacrificeSelf,
                CostPart::PayLife(1),
            ],
        }, &[Effect::SearchLibrary {
            filter: &SEARCH_FILTER,
            finds: &[Find::BATTLEFIELD_TAPPED],
            optional: false,
        }])],
}

// Fetchland family coverage lives in baylee-engine (fetchland test with
// Polluted Delta + the land-wave group test).
