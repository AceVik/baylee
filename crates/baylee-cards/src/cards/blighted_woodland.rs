//! Blighted Woodland — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}{G}, {T}, Sacrifice this land: Search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle.
//! Set: CLB #881 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 94e0ad38-31b3-46ba-9999-9b6ff57906ae | Oracle ID: 02679a2e-303d-412f-87d8-0a37a8ca259c
// IMPLEMENTED — tap for {C}, or {3}{G} + tap + sacrifice to search for up to two basic lands onto the battlefield tapped.

use baylee_cards_dsl::prelude::*;

card! {
    index: 284,
    oracle_id: "02679a2e-303d-412f-87d8-0a37a8ca259c",
    scryfall_id: "94e0ad38-31b3-46ba-9999-9b6ff57906ae",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Blighted Woodland",
        types: TypeSet::LAND,
    },
    ],
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            Cost {
                mana: baylee_core::mana!("{3}{G}"),
                parts: &[CostPart::TapSelf, CostPart::SacrificeSelf],
            },
            &[Effect::SearchLibrary {
                filter: &Filter::BASIC_LAND,
                finds: &[Find::BATTLEFIELD_TAPPED, Find::BATTLEFIELD_TAPPED],
                optional: true,
            }],
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 284);
        assert_eq!(CARD.oracle_id, "02679a2e-303d-412f-87d8-0a37a8ca259c");
        assert_eq!(CARD.scryfall_id, "94e0ad38-31b3-46ba-9999-9b6ff57906ae");
        assert_eq!(CARD.faces[0].name, "Blighted Woodland");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.color_identity, ColorSet::from_slice(&[Color::Green]));
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 2);
    }
}

// Engine-level coverage lives in baylee-engine (search_tests): search for up
// to two basic land cards onto the battlefield tapped, and shuffle.
