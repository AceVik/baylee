//! Arid Mesa — (no cost) — Land
//! Oracle: {T}, Pay 1 life, Sacrifice this land: Search your library for a Mountain or Plains card, put it onto the battlefield, then shuffle.
//! Set: MH2 #244 — Modern Horizons 2 | Scryfall ID: 25ac5405-df7b-4097-914a-022cb18e20d4 | Oracle ID: c5acf2a5-40f4-433d-a74d-1cb56c521464
// IMPLEMENTED — fetchland (tap + pay life + sacrifice → search Mountain/Plains
// to the battlefield tapped, shuffle).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SEARCH_FILTER: Filter = Filter::Or(&[
    Filter::HasSubtype(land::MOUNTAIN),
    Filter::HasSubtype(land::PLAINS),
]);

card! {
    index: 7,
    oracle_id: "c5acf2a5-40f4-433d-a74d-1cb56c521464",
    scryfall_id: "25ac5405-df7b-4097-914a-022cb18e20d4",
    faces: &[face! {
        name: "Arid Mesa",
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
            finds: &[Find::BATTLEFIELD],
            optional: false,
        }])],
}

// Fetchland family coverage lives in baylee-engine (fetchland test with
// Polluted Delta + the land-wave group test).
