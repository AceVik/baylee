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
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::PHYREXIAN, creature::SHAPESHIFTER],
        power: Some(0),
        toughness: Some(0),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&ARTIFACT_OR_CREATURE),
        mods: &[CopyMod::AddType(TypeSet::ARTIFACT)],
    }],
};

#[cfg(test)]
mod tests {}
