//! Buried Ruin — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Sacrifice this land: Return target artifact card from your graveyard to your hand.
//! Set: EOC #150 — Edge of Eternities Commander | Scryfall ID: 7ab1a074-9d92-4125-9200-84cd035f8f53 | Oracle ID: 3644f316-f9a3-46c9-9b1e-747f86cf4ead
// IMPLEMENTED — tap for {C} + {2}, tap, sacrifice self to return target artifact from graveyard to hand.

use baylee_cards_dsl::prelude::*;

card! {
    index: 318,
    oracle_id: "3644f316-f9a3-46c9-9b1e-747f86cf4ead",
    scryfall_id: "7ab1a074-9d92-4125-9200-84cd035f8f53",
    faces: &[
    face! {
        name: "Buried Ruin",
        types: TypeSet::LAND,
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            Cost {
                mana: baylee_core::mana!("{2}"),
                parts: &[CostPart::TapSelf, CostPart::SacrificeSelf],
            },
            &[Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&Filter::ARTIFACT, PlayerRel::You),
            }],
            target: Some(TargetSpec::CardInGraveyard(&Filter::ARTIFACT, PlayerRel::You)),
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 318);
        assert_eq!(CARD.oracle_id, "3644f316-f9a3-46c9-9b1e-747f86cf4ead");
        assert_eq!(CARD.scryfall_id, "7ab1a074-9d92-4125-9200-84cd035f8f53");
        assert_eq!(CARD.faces[0].name, "Buried Ruin");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 2);
    }
}

// Engine-level coverage: mana ability produces {C} (basics.rs); activation
// pays {2}, tap and sacrifices self to return target artifact card from graveyard to hand (zones.rs).
