//! Evolving Wilds — (no cost) — Land
//! Oracle: {T}, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.
//! Set: MSC #240 — Marvel Super Heroes Commander | Scryfall ID: c0318a48-30e4-4ef7-be3d-5e561c5ce428 | Oracle ID: a75445d3-1303-4bb5-89ad-26ea93fecd48
// IMPLEMENTED - {T}, sacrifice: one basic land onto the battlefield tapped.
//
// "Basic land card" is the *supertype* (CR 205.4a), which is what
// `Filter::BASIC_LAND` says. Written as a subtype test it would also find
// Breeding Pool, which carries the Forest subtype and is not a basic land.
// Tapped is the difference from a fetchland: `Find::BATTLEFIELD_TAPPED`.

use baylee_cards_dsl::prelude::*;

card! {
    index: 480,
    oracle_id: "a75445d3-1303-4bb5-89ad-26ea93fecd48",
    scryfall_id: "c0318a48-30e4-4ef7-be3d-5e561c5ce428",
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Evolving Wilds",
        types: TypeSet::LAND,
    },
    ],
    abilities: &[activated!(
        Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::TapSelf, CostPart::SacrificeSelf],
        },
        &[Effect::SearchLibrary {
            filter: &Filter::BASIC_LAND,
            finds: &[Find::BATTLEFIELD_TAPPED],
            optional: false,
        }]
    )],
}
