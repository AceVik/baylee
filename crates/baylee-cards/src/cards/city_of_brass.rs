//! City of Brass — (no cost) — Land
//! Oracle: Whenever this land becomes tapped, it deals 1 damage to you.
//! Oracle: {T}: Add one mana of any color.
//! Set: TMC #62 — Teenage Mutant Ninja Turtles Eternal | Scryfall ID: c21565d0-fc40-4d89-9b27-87c03385e0af | Oracle ID: f25351e3-539b-4bbc-b92d-6480acf4d722
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(20),
    oracle_id: "f25351e3-539b-4bbc-b92d-6480acf4d722",
    scryfall_id: "c21565d0-fc40-4d89-9b27-87c03385e0af",
    faces: &[FaceDef {
        name: "City of Brass",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
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
