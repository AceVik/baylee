//! Reflecting Pool — (no cost) — Land
//! Oracle: {T}: Add one mana of any type that a land you control could produce.
//! Set: CLB #358 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 18a1b3f5-473d-45ca-be0d-e67e77ba30ce | Oracle ID: 67f43ac6-2a58-4b53-b5d7-0330e2a252e2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(128),
    oracle_id: "67f43ac6-2a58-4b53-b5d7-0330e2a252e2",
    scryfall_id: "18a1b3f5-473d-45ca-be0d-e67e77ba30ce",
    faces: &[FaceDef {
        name: "Reflecting Pool",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
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
