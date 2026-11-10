//! Buried Ruin — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Sacrifice this land: Return target artifact card from your graveyard to your hand.
//! Set: EOC #150 — Edge of Eternities Commander | Scryfall ID: 7ab1a074-9d92-4125-9200-84cd035f8f53 | Oracle ID: 3644f316-f9a3-46c9-9b1e-747f86cf4ead
// IMPLEMENTED — {T}: Add {C}, and {2}, {T}, Sacrifice: Return target artifact card from graveyard to hand.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BURIED_RUIN,
    oracle_id = "3644f316-f9a3-46c9-9b1e-747f86cf4ead",
    scryfall_id = "7ab1a074-9d92-4125-9200-84cd035f8f53",
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Buried Ruin", types = TypeSet::LAND,),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf, SacrificeSelf),
            &[Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&Filter::ARTIFACT, PlayerRel::You),
            }],
            target = Some(TargetSpec::CardInGraveyard(
                &Filter::ARTIFACT,
                PlayerRel::You
            )),
        ),
    ],
);
