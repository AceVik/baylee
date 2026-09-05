//! Cabaretti Courtyard — (no cost) — Land
//! Oracle: When this land enters, sacrifice it. When you do, search your library for a basic Mountain, Forest, or Plains card, put it onto the battlefield tapped, then shuffle and you gain 1 life.
//! Set: EOC #151 — Edge of Eternities Commander | Scryfall ID: c54ddd4e-f668-4ec8-b123-59afa977eba4 | Oracle ID: 65424bea-fd53-4f85-9757-0b91a6d40ba4
// IMPLEMENTED — ETB sacrifice to fetch a basic Mountain, Forest, or Plains tapped, gain 1 life.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SEARCH_FILTER: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::BASIC),
    Filter::Or(&[
        Filter::HasSubtype(land::MOUNTAIN),
        Filter::HasSubtype(land::FOREST),
        Filter::HasSubtype(land::PLAINS),
    ]),
]);

card! {
    index: 322,
    oracle_id: "65424bea-fd53-4f85-9757-0b91a6d40ba4",
    scryfall_id: "c54ddd4e-f668-4ec8-b123-59afa977eba4",
    faces: &[
    face! {
        name: "Cabaretti Courtyard",
        types: TypeSet::LAND,
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[triggered!(
        Trigger::EntersBattlefield(&Filter::This),
        &[
            Effect::SacrificeSelf,
            Effect::SearchLibrary {
                filter: &SEARCH_FILTER,
                finds: &[Find::BATTLEFIELD_TAPPED],
                optional: false,
            },
            Effect::GainLife {
                amount: Amount::Fixed(1),
            },
        ],
    )],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 322);
        assert_eq!(CARD.oracle_id, "65424bea-fd53-4f85-9757-0b91a6d40ba4");
        assert_eq!(CARD.scryfall_id, "c54ddd4e-f668-4ec8-b123-59afa977eba4");
        assert_eq!(CARD.faces[0].name, "Cabaretti Courtyard");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 1);
    }
}

// Engine-level coverage: ETB trigger sacrifices this land, searches library
// for a basic Mountain, Forest, or Plains onto the battlefield tapped, shuffles,
// and gains 1 life.
