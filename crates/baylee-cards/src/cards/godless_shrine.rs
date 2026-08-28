//! Godless Shrine — (no cost) — Land — Plains Swamp
//! Oracle: ({T}: Add {W} or {B}.)
//! Oracle: As this land enters, you may pay 2 life. If you don't, it enters tapped.
//! Set: TRK #285 — Star Trek | Scryfall ID: 8fbd1ae0-3d4c-492a-a1ea-85a95fa3d7b6 | Oracle ID: 73864fcc-1bde-4bc0-831e-2b93e546e417
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(61),
    oracle_id: "73864fcc-1bde-4bc0-831e-2b93e546e417",
    scryfall_id: "8fbd1ae0-3d4c-492a-a1ea-85a95fa3d7b6",
    faces: &[FaceDef {
        name: "Godless Shrine",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::land::PLAINS, subtypes::land::SWAMP],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::White]),
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
