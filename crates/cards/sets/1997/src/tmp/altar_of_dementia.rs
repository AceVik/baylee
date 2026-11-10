//! Altar of Dementia — {2} — Artifact
//! Oracle: Sacrifice a creature: Target player mills cards equal to the sacrificed creature's power.
//! Set: MH1 #218 — Modern Horizons | Scryfall ID: 169356e0-46dc-4096-8e66-36726454f104 | Oracle ID: d64e9152-ef24-4394-aeb0-9c3befc56549
// IMPLEMENTED — Sacrifices a creature you control for target player to mill cards equal to the sacrificed creature's power.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ALTAR_OF_DEMENTIA,
    oracle_id = "d64e9152-ef24-4394-aeb0-9c3befc56549",
    scryfall_id = "169356e0-46dc-4096-8e66-36726454f104",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Altar of Dementia",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!(Sacrifice(&Filter::YOUR_CREATURE)),
        &[Effect::Mill {
            amount: Amount::SacrificedPower,
            target: PlayerRel::Chosen,
        }],
        target = Some(TargetSpec::AnyPlayer),
    )],
);
