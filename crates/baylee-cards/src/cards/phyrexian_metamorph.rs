//! Phyrexian Metamorph — {3}{U/P} — Artifact Creature — Phyrexian Shapeshifter
//! Oracle: ({U/P} can be paid with either {U} or 2 life.)
//! Oracle: You may have this creature enter as a copy of any artifact or creature on the battlefield, except it's an artifact in addition to its other types.
//! Set: EOC #75 — Edge of Eternities Commander | Scryfall ID: a564c2e8-f49f-4ed7-850f-7c8bc92e4926 | Oracle ID: 340bbe8b-e987-4c3e-ab4e-9dee63e57d4f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(114),
    oracle_id: "340bbe8b-e987-4c3e-ab4e-9dee63e57d4f",
    scryfall_id: "a564c2e8-f49f-4ed7-850f-7c8bc92e4926",
    faces: &[FaceDef {
        name: "Phyrexian Metamorph",
        mana_cost: baylee_core::mana!("{3}{U/P}"),
        types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::creature::PHYREXIAN,
            subtypes::creature::SHAPESHIFTER,
        ],
        power: Some(0),
        toughness: Some(0),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Unimplemented,
    abilities: &[],
};

#[cfg(test)]
mod tests {
    // TODO(card): implement abilities + tests, see docs/card-dsl.md.
}
