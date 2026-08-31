//! Phyrexian Metamorph — {3}{U/P} — Artifact Creature — Phyrexian Shapeshifter
//! Oracle: ({U/P} can be paid with either {U} or 2 life.)
//! Oracle: You may have this creature enter as a copy of any artifact or creature on the battlefield, except it's an artifact in addition to its other types.
//! Set: EOC #75 — Edge of Eternities Commander | Scryfall ID: a564c2e8-f49f-4ed7-850f-7c8bc92e4926 | Oracle ID: 340bbe8b-e987-4c3e-ab4e-9dee63e57d4f
// IMPLEMENTED — clone with artifact addition; Phyrexian mana payment with
// life is an auto-pay preference for now (payment plans M3).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, CopyMod, Coverage, FaceDef, Filter, KeywordSet,
    PartnerKind, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ARTIFACT_OR_CREATURE: Filter = Filter::Or(&[
    Filter::HasType(TypeSet::ARTIFACT),
    Filter::HasType(TypeSet::CREATURE),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(114),
    oracle_id: "340bbe8b-e987-4c3e-ab4e-9dee63e57d4f",
    scryfall_id: "a564c2e8-f49f-4ed7-850f-7c8bc92e4926",
    faces: &[FaceDef {
        name: "Phyrexian Metamorph",
        mana_cost: baylee_core::mana!("{3}{U/P}"),
        types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes: &[creature::PHYREXIAN, creature::SHAPESHIFTER],
        power: Some(0),
        toughness: Some(0),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&ARTIFACT_OR_CREATURE),
        mods: &[CopyMod::AddType(TypeSet::ARTIFACT)],
    }],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
