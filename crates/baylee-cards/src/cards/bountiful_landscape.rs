//! Bountiful Landscape — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Sacrifice this land: Search your library for a basic Forest, Island, or Mountain card, put it onto the battlefield tapped, then shuffle.
//! Oracle: Cycling {G}{U}{R} ({G}{U}{R}, Discard this card: Draw a card.)
//! Set: TDC #342 — Tarkir: Dragonstorm Commander | Scryfall ID: f1c99617-6e38-4af7-a064-6db9a5e2cc6a | Oracle ID: 2eb69a8f-9456-4852-b868-85ae609d3441
// IMPLEMENTED — tap for {C}, fetch basic Forest/Island/Mountain tapped, cycling {G}{U}{R}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SEARCH_FILTER: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::BASIC),
    Filter::Or(&[
        Filter::HasSubtype(land::FOREST),
        Filter::HasSubtype(land::ISLAND),
        Filter::HasSubtype(land::MOUNTAIN),
    ]),
]);

card! {
    index: 305,
    oracle_id: "2eb69a8f-9456-4852-b868-85ae609d3441",
    scryfall_id: "f1c99617-6e38-4af7-a064-6db9a5e2cc6a",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Red, Color::Blue]),
    faces: &[
    face! {
        name: "Bountiful Landscape",
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
                mana: baylee_core::mana!("{G}{U}{R}"),
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
        assert_eq!(CARD.index.get(), 305);
        assert_eq!(CARD.oracle_id, "2eb69a8f-9456-4852-b868-85ae609d3441");
        assert_eq!(CARD.scryfall_id, "f1c99617-6e38-4af7-a064-6db9a5e2cc6a");
        assert_eq!(CARD.faces[0].name, "Bountiful Landscape");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(
            CARD.color_identity,
            ColorSet::from_slice(&[Color::Green, Color::Red, Color::Blue])
        );
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 3);
    }
}

// Engine-level coverage: mana ability produces {C} (basics.rs); fetchland
// search for basic Forest, Island, or Mountain onto the battlefield tapped
// (search_tests.rs); cycling from hand via CostPart::DiscardSelf (abilities.rs).
