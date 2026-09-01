//! Phantasmal Image — {1}{U} — Creature — Illusion
//! Oracle: You may have this creature enter as a copy of any creature on the battlefield, except it's an Illusion in addition to its other types and it has "When this creature becomes the target of a spell or ability, sacrifice it."
//! Set: AFC #89 — Forgotten Realms Commander | Scryfall ID: c1c080cf-a5e8-4d9d-af49-f78588971e87 | Oracle ID: bde94af8-faea-41ff-8eed-ba642eac9968
// IMPLEMENTED — clone + sacrifice-when-targeted.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, CopyMod, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_CREATURE: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(113),
    oracle_id: "bde94af8-faea-41ff-8eed-ba642eac9968",
    scryfall_id: "c1c080cf-a5e8-4d9d-af49-f78588971e87",
    faces: &[FaceDef {
        name: "Phantasmal Image",
        mana_cost: baylee_core::mana!("{1}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::ILLUSION],
        power: Some(0),
        toughness: Some(0),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::CopyOnEnter {
            target: TargetSpec::Object(&ANY_CREATURE),
            mods: &[],
        },
        AbilityDef::Triggered {
            trigger: Trigger::BecomesTarget,
            once_per_turn: false,
            effects: &[Effect::SacrificeSelf],
            targets: None,
        },
    ],
    ..CardDef::DEFAULT
};
