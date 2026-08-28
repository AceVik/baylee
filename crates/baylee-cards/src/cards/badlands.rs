//! Badlands — (no cost) — Land — Swamp Mountain
//! Oracle: ({T}: Add {B} or {R}.)
//! Set: VMA #291 — Vintage Masters | Scryfall ID: 73403d04-fe97-4830-8b80-16dd1a1a6cc1 | Oracle ID: 13ff3222-91cb-4796-a34e-899ed817694c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(9),
    oracle_id: "13ff3222-91cb-4796-a34e-899ed817694c",
    scryfall_id: "73403d04-fe97-4830-8b80-16dd1a1a6cc1",
    faces: &[FaceDef {
        name: "Badlands",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::land::SWAMP, subtypes::land::MOUNTAIN],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Red]),
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
