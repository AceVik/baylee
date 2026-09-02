//! Bojuka Bog — (no cost) — Land
//! Oracle: Bojuka Bog enters the battlefield tapped.
//! When Bojuka Bog enters, exile all cards from target player's graveyard.
//! Set: C18 #259 — Commander 2018 | Scryfall ID: 55b5b094-9d2d-4d96-b90c-78fecdae725a | Oracle ID: 04b7362d-0490-4cb0-b5d7-2a7732f659ce
// IMPLEMENTED — ETB tapped + exile target player's graveyard (opponent
// auto-resolves heads-up; multiplayer player choice is a protocol M3 item).

use baylee_cards_dsl::prelude::*;

card! {
    index: 14,
    oracle_id: "04b7362d-0490-4cb0-b5d7-2a7732f659ce",
    scryfall_id: "55b5b094-9d2d-4d96-b90c-78fecdae725a",
    faces: &[face! {
        name: "Bojuka Bog",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::Tapped],
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::ExileGraveyard {
                player: PlayerRel::Chosen,
            }], targets: Some(TargetReq::one(
                TargetSpec::AnyPlayer,
            ))),
    ],
}
