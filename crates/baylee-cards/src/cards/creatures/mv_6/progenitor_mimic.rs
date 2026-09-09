//! Progenitor Mimic — {4}{G}{U} — Creature — Shapeshifter
//! Oracle: You may have this creature enter as a copy of any creature on the battlefield, except it has "At the beginning of your upkeep, if this creature isn't a token, create a token that's a copy of this creature."
//! Set: 2XM #212 — Double Masters | Scryfall ID: acba72e1-3f7f-4e5c-af3f-dfe37b5d61f9 | Oracle ID: 88929ea9-900f-4dbb-b16c-cf3bad4e410c
// IMPLEMENTED — clone + upkeep token-copy-of-self.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card! {
    index: 121,
    oracle_id: "88929ea9-900f-4dbb-b16c-cf3bad4e410c",
    scryfall_id: "acba72e1-3f7f-4e5c-af3f-dfe37b5d61f9",
    faces: &[face! {
        name: "Progenitor Mimic",
        mana_cost: baylee_core::mana!("{4}{G}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::SHAPESHIFTER],
        power: Some(0),
        toughness: Some(0),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Green]),
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::CopyOnEnter {
            target: TargetSpec::Object(&Filter::CREATURE),
            mods: &[],
        },
        triggered!(Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            }, &[Effect::CreateTokenCopyOf {
                target: None,
                kicked_bonus: 0,
            }]),
    ],
}
