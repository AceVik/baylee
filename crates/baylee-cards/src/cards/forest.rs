//! Forest — (no cost) — Basic Land — Forest
//! Oracle: ({T}: Add {G}.)
//! Set: TRK #325 — Star Trek | Scryfall ID: dce15387-4114-4b3e-91aa-5b42b45c44ac | Oracle ID: b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(55),
    oracle_id: "b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6",
    scryfall_id: "dce15387-4114-4b3e-91aa-5b42b45c44ac",
    faces: &[FaceDef {
        name: "Forest",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::BASIC,
        subtypes: &[subtypes::land::FOREST],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Green]),
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
