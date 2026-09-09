//! Terramorphic Expanse — (no cost) — Land
//! Oracle: {T}, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.
//! Set: MSC #273 — Marvel Super Heroes Commander | Scryfall ID: a81f924b-0527-4311-8120-9bfff71524f6 | Oracle ID: 1bd3e453-aa21-4ee6-95c2-d6d920ee8e7a
// IMPLEMENTED - Evolving Wilds under another name, and printed as one: same
// cost, same search, same tapped arrival. Nothing here may be shared with it
// - two cards that happen to read alike are still two cards.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1161,
    oracle_id: "1bd3e453-aa21-4ee6-95c2-d6d920ee8e7a",
    scryfall_id: "a81f924b-0527-4311-8120-9bfff71524f6",
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Terramorphic Expanse",
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
