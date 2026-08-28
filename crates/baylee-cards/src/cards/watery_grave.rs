//! Watery Grave — (no cost) — Land — Island Swamp
//! Oracle: ({T}: Add {U} or {B}.)
//! Oracle: As this land enters, you may pay 2 life. If you don't, it enters tapped.
//! Set: TRK #306 — Star Trek | Scryfall ID: 5525d6a6-e532-4047-9da4-bfae7927fecc | Oracle ID: fc9ec820-4245-4a96-b009-5308a818ca58
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(189),
    oracle_id: "fc9ec820-4245-4a96-b009-5308a818ca58",
    scryfall_id: "5525d6a6-e532-4047-9da4-bfae7927fecc",
    faces: &[FaceDef {
        name: "Watery Grave",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::land::ISLAND, subtypes::land::SWAMP],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Blue]),
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
