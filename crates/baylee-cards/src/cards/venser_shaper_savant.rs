//! Venser, Shaper Savant — {2}{U}{U} — Legendary Creature — Human Wizard
//! Oracle: Flash (You may cast this spell any time you could cast an instant.)
//! Oracle: When Venser enters, return target spell or permanent to its owner's hand.
//! Set: 2X2 #66 — Double Masters 2022 | Scryfall ID: 77e19416-aa6c-46f1-b247-a94da5d1a13a | Oracle ID: 0f41cefc-d6ff-4db7-ba35-502b7e081de1
// IMPLEMENTED — flash + ETB bounce of a spell or permanent.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card! {
    index: 182,
    oracle_id: "0f41cefc-d6ff-4db7-ba35-502b7e081de1",
    scryfall_id: "77e19416-aa6c-46f1-b247-a94da5d1a13a",
    faces: &[face! {
        name: "Venser, Shaper Savant",
        mana_cost: baylee_core::mana!("{2}{U}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[creature::HUMAN, creature::WIZARD],
        power: Some(2),
        toughness: Some(2),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::FLASH,
    commander: CommanderRule::Legendary,
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::ReturnToHand {
            target: TargetSpec::StackOrBattlefield(&Filter::Any),
        }], targets: Some(TargetReq::one(TargetSpec::StackOrBattlefield(
            &Filter::Any,
        ))))],
}
