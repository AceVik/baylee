//! Plains — (no cost) — Basic Land — Plains
//! Oracle: ({T}: Add {W}.)
//! Set: TRK #317 — Star Trek | Scryfall ID: 8ab0f4c0-b331-4c57-b68f-2e24bb5ba06c | Oracle ID: bc71ebf6-2056-41f7-be35-b2e5c34afa99
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(115),
    oracle_id: "bc71ebf6-2056-41f7-be35-b2e5c34afa99",
    scryfall_id: "8ab0f4c0-b331-4c57-b68f-2e24bb5ba06c",
    faces: &[FaceDef {
        name: "Plains",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::BASIC,
        subtypes: &[subtypes::land::PLAINS],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
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
