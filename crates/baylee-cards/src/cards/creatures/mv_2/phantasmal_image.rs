//! Phantasmal Image — {1}{U} — Creature — Illusion
//! Oracle: You may have this creature enter as a copy of any creature on the battlefield, except it's an Illusion in addition to its other types and it has "When this creature becomes the target of a spell or ability, sacrifice it."
//! Set: AFC #89 — Forgotten Realms Commander | Scryfall ID: c1c080cf-a5e8-4d9d-af49-f78588971e87 | Oracle ID: bde94af8-faea-41ff-8eed-ba642eac9968
// IMPLEMENTED — clone + sacrifice-when-targeted.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card! {
    index: 113,
    oracle_id: "bde94af8-faea-41ff-8eed-ba642eac9968",
    scryfall_id: "c1c080cf-a5e8-4d9d-af49-f78588971e87",
    faces: &[face! {
        name: "Phantasmal Image",
        mana_cost: baylee_core::mana!("{1}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::ILLUSION],
        power: Some(0),
        toughness: Some(0),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::CopyOnEnter {
            target: TargetSpec::Object(&Filter::CREATURE),
            mods: &[],
        },
        triggered!(Trigger::BecomesTarget, &[Effect::SacrificeSelf]),
    ],
}
