//! Scalding Tarn — (no cost) — Land
//! Oracle: {T}, Pay 1 life, Sacrifice this land: Search your library for a Island or Mountain card, put it onto the battlefield, then shuffle.
//! Set: MH2 #254 — Modern Horizons 2 | Scryfall ID: 71e491c5-8c07-449b-b2f1-ffa052e6d311 | Oracle ID: cb027150-848c-4a66-88ad-e20222304dd8
// IMPLEMENTED — fetchland (tap + pay life + sacrifice → search Island/Mountain
// to the battlefield tapped, shuffle).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SEARCH_FILTER: Filter = Filter::Or(&[
    Filter::HasSubtype(land::ISLAND),
    Filter::HasSubtype(land::MOUNTAIN),
]);

card! {
    index: 139,
    oracle_id: "cb027150-848c-4a66-88ad-e20222304dd8",
    scryfall_id: "71e491c5-8c07-449b-b2f1-ffa052e6d311",
    faces: &[face! {
        name: "Scalding Tarn",
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
