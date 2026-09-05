//! Deceptive Landscape — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Sacrifice this land: Search your library for a basic Plains, Swamp, or Forest card, put it onto the battlefield tapped, then shuffle.
//! Oracle: Cycling {W}{B}{G} ({W}{B}{G}, Discard this card: Draw a card.)
//! Set: TDC #356 — Tarkir: Dragonstorm Commander | Scryfall ID: ba863744-929b-43f3-a773-342ba3437b18 | Oracle ID: 1831fe12-dbe0-437f-8fc8-f01bbb701fe1
// IMPLEMENTED — tap for {C}, fetch basic Plains/Swamp/Forest tapped, cycling {W}{B}{G}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SEARCH_FILTER: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::BASIC),
    Filter::Or(&[
        Filter::HasSubtype(land::PLAINS),
        Filter::HasSubtype(land::SWAMP),
        Filter::HasSubtype(land::FOREST),
    ]),
]);

card! {
    index: 410,
    oracle_id: "1831fe12-dbe0-437f-8fc8-f01bbb701fe1",
    scryfall_id: "ba863744-929b-43f3-a773-342ba3437b18",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Green, Color::White]),
    faces: &[
    face! {
        name: "Deceptive Landscape",
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
                mana: baylee_core::mana!("{W}{B}{G}"),
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
        assert_eq!(CARD.index.get(), 410);
        assert_eq!(CARD.oracle_id, "1831fe12-dbe0-437f-8fc8-f01bbb701fe1");
        assert_eq!(CARD.scryfall_id, "ba863744-929b-43f3-a773-342ba3437b18");
        assert_eq!(CARD.faces[0].name, "Deceptive Landscape");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(
            CARD.color_identity,
            ColorSet::from_slice(&[Color::Black, Color::Green, Color::White])
        );
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 3);
    }
}

// Engine-level coverage: mana ability produces {C} (basics.rs); fetchland
// search for basic Plains, Swamp, or Forest onto the battlefield tapped
// (search_tests.rs); cycling from hand via CostPart::DiscardSelf (abilities.rs).
