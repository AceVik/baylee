//! Bant Panorama — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}, Sacrifice this land: Search your library for a basic Forest, Plains, or Island card, put it onto the battlefield tapped, then shuffle.
//! Set: NCC #387 — New Capenna Commander | Scryfall ID: dc506497-4ecd-41d3-9042-f92f498251b0 | Oracle ID: 0a1d817d-dce8-4e83-a380-909f7c9eee46
// IMPLEMENTED — tap for {C}, or {1} + tap + sacrifice to search for a basic Forest, Plains, or Island onto the battlefield tapped.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SEARCH_FILTER: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::BASIC),
    Filter::Or(&[
        Filter::HasSubtype(land::FOREST),
        Filter::HasSubtype(land::PLAINS),
        Filter::HasSubtype(land::ISLAND),
    ]),
]);

card! {
    index: 257,
    oracle_id: "0a1d817d-dce8-4e83-a380-909f7c9eee46",
    scryfall_id: "dc506497-4ecd-41d3-9042-f92f498251b0",
    faces: &[
    face! {
        name: "Bant Panorama",
        types: TypeSet::LAND,
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            Cost {
                mana: baylee_core::mana!("{1}"),
                parts: &[CostPart::TapSelf, CostPart::SacrificeSelf],
            },
            &[Effect::SearchLibrary {
                filter: &SEARCH_FILTER,
                finds: &[Find::BATTLEFIELD_TAPPED],
                optional: false,
            }],
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 257);
        assert_eq!(CARD.oracle_id, "0a1d817d-dce8-4e83-a380-909f7c9eee46");
        assert_eq!(CARD.scryfall_id, "dc506497-4ecd-41d3-9042-f92f498251b0");
        assert_eq!(CARD.faces[0].name, "Bant Panorama");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 2);
    }
}
