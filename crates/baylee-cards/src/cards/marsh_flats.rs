//! Marsh Flats — (no cost) — Land
//! Oracle: {T}, Pay 1 life, Sacrifice this land: Search your library for a Plains or Swamp card, put it onto the battlefield, then shuffle.
//! Set: MH2 #248 — Modern Horizons 2 | Scryfall ID: 9db3ba6d-eb7f-4f5b-9a3b-c6239c3baa42 | Oracle ID: dab520d0-20b4-4273-ba6b-eb07f85ea433
// IMPLEMENTED — fetchland (tap + pay life + sacrifice → search Plains/Swamp
// to the battlefield tapped, shuffle).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SEARCH_FILTER: Filter = Filter::Or(&[
    Filter::HasSubtype(land::PLAINS),
    Filter::HasSubtype(land::SWAMP),
]);

card! {
    index: 91,
    oracle_id: "dab520d0-20b4-4273-ba6b-eb07f85ea433",
    scryfall_id: "9db3ba6d-eb7f-4f5b-9a3b-c6239c3baa42",
    faces: &[face! {
        name: "Marsh Flats",
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
