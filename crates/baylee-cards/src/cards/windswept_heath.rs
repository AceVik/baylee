//! Windswept Heath — (no cost) — Land
//! Oracle: {T}, Pay 1 life, Sacrifice this land: Search your library for a Forest or Plains card, put it onto the battlefield, then shuffle.
//! Set: MH3 #235 — Modern Horizons 3 | Scryfall ID: bd1d13f7-fd38-4f0b-a8e0-1eac78668117 | Oracle ID: 29737a60-3ebd-40d9-b935-c4f54b90d45d
// IMPLEMENTED — fetchland (tap + pay life + sacrifice → search Forest/Plains
// to the battlefield tapped, shuffle).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SEARCH_FILTER: Filter = Filter::Or(&[
    Filter::HasSubtype(land::FOREST),
    Filter::HasSubtype(land::PLAINS),
]);

card! {
    index: 191,
    oracle_id: "29737a60-3ebd-40d9-b935-c4f54b90d45d",
    scryfall_id: "bd1d13f7-fd38-4f0b-a8e0-1eac78668117",
    faces: &[face! {
        name: "Windswept Heath",
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
