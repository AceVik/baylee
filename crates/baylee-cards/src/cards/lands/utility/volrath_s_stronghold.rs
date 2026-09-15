//! Volrath's Stronghold — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}{B}, {T}: Put target creature card from your graveyard on top of your library.
//! Set: TPR #248 — Tempest Remastered | Scryfall ID: f465ae5f-61f0-42c4-978f-841ba1226f56 | Oracle ID: 73b8cf90-3c71-4f8b-a29f-61894b7f27c9
// IMPLEMENTED.

use baylee_cards_dsl::prelude::*;

card! {
    index: 186,
    oracle_id: "73b8cf90-3c71-4f8b-a29f-61894b7f27c9",
    scryfall_id: "f465ae5f-61f0-42c4-978f-841ba1226f56",
    faces: &[face! {
        name: "Volrath's Stronghold",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // `{1}{B}, {T}`, not `{T}`. The header this file carried had dropped
        // the mana from the printed sentence, and the code was written from
        // the header: a free, repeatable recursion of any creature in your
        // graveyard, which is a different card.
        activated!(Cost { mana: mana!("{1}{B}"), parts: &[CostPart::TapSelf] }, &[Effect::GraveyardToTop {
                target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            }], target: Some(TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You))),
    ],
}
