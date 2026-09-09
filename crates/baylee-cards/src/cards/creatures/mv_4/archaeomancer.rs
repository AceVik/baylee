//! Archaeomancer — {2}{U}{U} — Creature — Human Wizard
//! Oracle: When this creature enters, return target instant or sorcery card from your graveyard to your hand.
//! Set: UMA #45 — Ultimate Masters | Scryfall ID: cc258713-6ce3-44e0-9b4b-8fa7d1d093a1 | Oracle ID: a91a3266-cadd-47a0-9b20-160307f14c07
// IMPLEMENTED — ETB spell recovery from your graveyard.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card! {
    index: 6,
    oracle_id: "a91a3266-cadd-47a0-9b20-160307f14c07",
    scryfall_id: "cc258713-6ce3-44e0-9b4b-8fa7d1d093a1",
    faces: &[face! {
        name: "Archaeomancer",
        mana_cost: baylee_core::mana!("{2}{U}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::HUMAN, creature::WIZARD],
        power: Some(1),
        toughness: Some(2),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::GraveyardToHand {
            target: TargetSpec::CardInGraveyard(&Filter::INSTANT_OR_SORCERY, PlayerRel::You),
        }], targets: Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::INSTANT_OR_SORCERY,
            PlayerRel::You,
        ))))],
}
