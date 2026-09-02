//! Bloodstained Mire — (no cost) — Land
//! Oracle: {T}, Pay 1 life, Sacrifice this land: Search your library for a Swamp or Mountain card, put it onto the battlefield, then shuffle.
//! Set: MH3 #216 — Modern Horizons 3 | Scryfall ID: 579743fe-f71e-4cb2-8629-d6b02ed1591d | Oracle ID: fc0707c7-d504-4ccf-a0d2-3eb6e26e7a57
// IMPLEMENTED — fetchland (tap + pay life + sacrifice → search Swamp/Mountain
// to the battlefield tapped, shuffle).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SEARCH_FILTER: Filter = Filter::Or(&[
    Filter::HasSubtype(land::SWAMP),
    Filter::HasSubtype(land::MOUNTAIN),
]);

card! {
    index: 13,
    oracle_id: "fc0707c7-d504-4ccf-a0d2-3eb6e26e7a57",
    scryfall_id: "579743fe-f71e-4cb2-8629-d6b02ed1591d",
    faces: &[face! {
        name: "Bloodstained Mire",
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
