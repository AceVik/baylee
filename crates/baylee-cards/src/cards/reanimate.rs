//! Reanimate — {B} — Sorcery
//! Oracle: Put target creature card from a graveyard onto the battlefield under your control. You lose life equal to that card's mana value.
//! Set: DSC #155 — Duskmourn: House of Horror Commander | Scryfall ID: 368b6903-5fc4-43e7-bd44-46b8107c8bb4 | Oracle ID: a044474a-cd72-4e9d-bd8d-a08f2de9cdc0
// IMPLEMENTED — reanimation with life payment equal to cmc.

use baylee_cards_dsl::prelude::*;

card! {
    index: 125,
    oracle_id: "a044474a-cd72-4e9d-bd8d-a08f2de9cdc0",
    scryfall_id: "368b6903-5fc4-43e7-bd44-46b8107c8bb4",
    faces: &[face! {
        name: "Reanimate",
        mana_cost: baylee_core::mana!("{B}"),
        types: TypeSet::SORCERY,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[
            Effect::GraveyardToBattlefield {
                target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::EachPlayer),
            },
            Effect::LoseLife {
                amount: Amount::TargetCmc,
                target: PlayerRel::You,
            },
        ], targets: Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::EachPlayer,
        ))))],
}
