//! Roaming Throne — {4} — Artifact Creature — Golem
//! Oracle: Ward {2}
//! Oracle: As this creature enters, choose a creature type.
//! Oracle: This creature is the chosen type in addition to its other types.
//! Oracle: If a triggered ability of another creature you control of the chosen type triggers, it triggers an additional time.
//! Set: LCI #258 — The Lost Caverns of Ixalan | Scryfall ID: 32fd8b7c-baf3-4d3d-be6f-044a917b11a0 | Oracle ID: 3640c29b-1534-4952-b297-619ade948431
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(136),
    oracle_id: "3640c29b-1534-4952-b297-619ade948431",
    scryfall_id: "32fd8b7c-baf3-4d3d-be6f-044a917b11a0",
    faces: &[FaceDef {
        name: "Roaming Throne",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::GOLEM],
        power: Some(4),
        toughness: Some(4),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::EMPTY,
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
