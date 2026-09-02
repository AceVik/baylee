//! Polluted Delta — (no cost) — Land
//! Oracle: {T}, Pay 1 life, Sacrifice this land: Search your library for an Island or Swamp card, put it onto the battlefield, then shuffle.
//! Set: MKC #246 — Commander: Murders at Karlov Manor | Scryfall ID: 6e288374-2b71-4ace-b1d2-a19fee6cb4af | Oracle ID: ef86989d-ce80-4e55-aece-7d11710eeffa
// IMPLEMENTED — fetchland (tap + pay life + sacrifice → search Island/Swamp
// to the battlefield tapped, shuffle).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SEARCH_FILTER: Filter = Filter::Or(&[
    Filter::HasSubtype(land::ISLAND),
    Filter::HasSubtype(land::SWAMP),
]);

card! {
    index: 116,
    oracle_id: "ef86989d-ce80-4e55-aece-7d11710eeffa",
    scryfall_id: "6e288374-2b71-4ace-b1d2-a19fee6cb4af",
    faces: &[face! {
        name: "Polluted Delta",
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

// Engine-level test lives in baylee-engine (fetchland_works): activation
// pays tap+sacrifice+1 life, offers only Island/Swamp options, puts the
// chosen card onto the battlefield tapped, and shuffles.
