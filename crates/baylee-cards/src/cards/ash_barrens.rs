//! Ash Barrens — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: Basic landcycling {1} ({1}, Discard this card: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.)
//! Set: TMC #60 — Teenage Mutant Ninja Turtles Eternal | Scryfall ID: e8510205-4baa-4f1c-bf64-5d931e2ddf48 | Oracle ID: 58257464-278e-45fa-8e0b-bcd9a7500bc1
// IMPLEMENTED — tap for {C} + basic landcycling {1}.

use baylee_cards_dsl::prelude::*;

card! {
    index: 240,
    oracle_id: "58257464-278e-45fa-8e0b-bcd9a7500bc1",
    scryfall_id: "e8510205-4baa-4f1c-bf64-5d931e2ddf48",
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Ash Barrens",
        types: TypeSet::LAND,
    },
    ],
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            Cost {
                mana: baylee_core::mana!("{1}"),
                parts: &[CostPart::DiscardSelf],
            },
            &[Effect::SearchLibrary {
                filter: &Filter::BASIC_LAND,
                finds: &[Find::HAND],
                optional: false,
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
        assert_eq!(CARD.index.get(), 240);
        assert_eq!(CARD.oracle_id, "58257464-278e-45fa-8e0b-bcd9a7500bc1");
        assert_eq!(CARD.scryfall_id, "e8510205-4baa-4f1c-bf64-5d931e2ddf48");
        assert_eq!(CARD.faces[0].name, "Ash Barrens");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 2);
    }
}

// Engine-level coverage: mana ability produces {C} (basics.rs); activation
// from hand via CostPart::DiscardSelf (abilities.rs); library search for
// a basic land card into hand with shuffle (search_tests.rs).
