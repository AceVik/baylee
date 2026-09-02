//! Flooded Strand — (no cost) — Land
//! Oracle: {T}, Pay 1 life, Sacrifice this land: Search your library for a Plains or Island card, put it onto the battlefield, then shuffle.
//! Set: MH3 #220 — Modern Horizons 3 | Scryfall ID: 8f85e12c-196b-4459-b81f-0c9c854e9f57 | Oracle ID: f3c7af78-a77d-4134-82a2-a5ce84285a84
// IMPLEMENTED — fetchland (tap + pay life + sacrifice → search Plains/Island
// to the battlefield tapped, shuffle).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SEARCH_FILTER: Filter = Filter::Or(&[
    Filter::HasSubtype(land::PLAINS),
    Filter::HasSubtype(land::ISLAND),
]);

card! {
    index: 52,
    oracle_id: "f3c7af78-a77d-4134-82a2-a5ce84285a84",
    scryfall_id: "8f85e12c-196b-4459-b81f-0c9c854e9f57",
    faces: &[face! {
        name: "Flooded Strand",
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
