//! Tishana's Tidebinder — {2}{U} — Creature — Merfolk Wizard
//! Oracle: Flash
//! Oracle: When this creature enters, counter up to one target activated or triggered ability. If an ability of an artifact, creature, or planeswalker is countered this way, that permanent loses all abilities for as long as this creature remains on the battlefield. (Mana abilities can't be targeted.)
//! Set: LCI #81 — The Lost Caverns of Ixalan | Scryfall ID: 907b3d1d-8c85-4707-80b5-c4d832df9846 | Oracle ID: 2993dc7d-723d-4a9b-94bd-4bb02a9f7243
// IMPLEMENTED — flash + counter target ability + ability suppression until EOT.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card! {
    index: 170,
    oracle_id: "2993dc7d-723d-4a9b-94bd-4bb02a9f7243",
    scryfall_id: "907b3d1d-8c85-4707-80b5-c4d832df9846",
    faces: &[face! {
        name: "Tishana's Tidebinder",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::MERFOLK, creature::WIZARD],
        power: Some(3),
        toughness: Some(2),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::FLASH,
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::EntersBattlefield(&Filter::This), &[
            Effect::CounterTargetAbility,
            Effect::TargetSourceLosesAbilities,
        ], targets: Some(TargetReq::one(TargetSpec::AbilityOnStack(&Filter::Any))))],
}
