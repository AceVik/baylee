//! Surgical Metamorph — {3}{U} — Artifact Creature — Phyrexian Shapeshifter
//! Oracle: This spell costs {1} less to cast if you weren't the starting player.
//! Oracle: You may have Surgical Metamorph enter as a copy of any permanent on the battlefield, except it's an artifact in addition to its other types.
//! Set: YONE #6 — Alchemy: Phyrexia | Scryfall ID: 1e7aa3a6-4219-4c54-97bd-571680af9e99 | Oracle ID: 4f328996-f9dd-4c7a-9548-bc4b9d0d943f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(161),
    oracle_id: "4f328996-f9dd-4c7a-9548-bc4b9d0d943f",
    scryfall_id: "1e7aa3a6-4219-4c54-97bd-571680af9e99",
    faces: &[FaceDef {
        name: "Surgical Metamorph",
        mana_cost: baylee_core::mana!("{3}{U}"),
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
