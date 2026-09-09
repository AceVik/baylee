//! Volrath's Stronghold — (no cost) — Land
//! Oracle: {T}: Add {C}. {T}: Put target creature card from your graveyard on top of your library.
//! Set: PD3 #352 — Premium Deck Series: Graveborn | Scryfall ID: f465ae5f-61f0-42c4-978f-841ba1226f56 | Oracle ID: 73b8cf90-3c71-4f8b-a29f-61894b7f27c9
// IMPLEMENTED.

use baylee_cards_dsl::prelude::*;

card! {
    index: 186,
    oracle_id: "73b8cf90-3c71-4f8b-a29f-61894b7f27c9",
    scryfall_id: "f465ae5f-61f0-42c4-978f-841ba1226f56",
    faces: &[face! {
        name: "Volrath's Stronghold",
        types: TypeSet::LAND,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(Cost::TAP, &[Effect::GraveyardToTop {
                target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            }], target: Some(TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You))),
    ],
}
