//! Contaminated Landscape — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Sacrifice this land: Search your library for a basic Plains, Island, or Swamp card, put it onto the battlefield tapped, then shuffle.
//! Oracle: Cycling {W}{U}{B} ({W}{U}{B}, Discard this card: Draw a card.)
//! Set: MH3 #218 — Modern Horizons 3 | Scryfall ID: e2312c49-1627-47ad-8113-78a999a97d8d | Oracle ID: 28196fd9-00c9-4cd0-b603-0eec8511ec79
// IMPLEMENTED — tap for {C}, fetch basic Plains/Island/Swamp tapped, cycling {W}{U}{B}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SEARCH_FILTER: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::BASIC),
    Filter::Or(&[
        Filter::HasSubtype(land::PLAINS),
        Filter::HasSubtype(land::ISLAND),
        Filter::HasSubtype(land::SWAMP),
    ]),
]);

card! {
    index: 372,
    oracle_id: "28196fd9-00c9-4cd0-b603-0eec8511ec79",
    scryfall_id: "e2312c49-1627-47ad-8113-78a999a97d8d",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White]),
    faces: &[
    face! {
        name: "Contaminated Landscape",
        types: TypeSet::LAND,
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            Cost {
                mana: ManaCost::ZERO,
                parts: &[CostPart::TapSelf, CostPart::SacrificeSelf],
            },
            &[Effect::SearchLibrary {
                filter: &SEARCH_FILTER,
                finds: &[Find::BATTLEFIELD_TAPPED],
                optional: false,
            }],
        ),
        activated!(
            Cost {
                mana: baylee_core::mana!("{W}{U}{B}"),
                parts: &[CostPart::DiscardSelf],
            },
            &[Effect::DrawCards {
                amount: Amount::Fixed(1),
            }],
            zone: ActivationZone::Hand,
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 372);
        assert_eq!(CARD.oracle_id, "28196fd9-00c9-4cd0-b603-0eec8511ec79");
        assert_eq!(CARD.scryfall_id, "e2312c49-1627-47ad-8113-78a999a97d8d");
        assert_eq!(CARD.faces[0].name, "Contaminated Landscape");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(
            CARD.color_identity,
            ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White])
        );
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 3);
    }
}

// Engine-level coverage: mana ability produces {C} (basics.rs); fetchland
// search for basic Plains, Island, or Swamp onto the battlefield tapped
// (search_tests.rs); cycling from hand via CostPart::DiscardSelf (abilities.rs).
