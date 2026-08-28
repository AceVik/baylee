//! Hallowed Fountain — (no cost) — Land — Plains Island
//! Oracle: ({T}: Add {W} or {U}.)
//! Oracle: As this land enters, you may pay 2 life. If you don't, it enters tapped.
//! Set: TRK #286 — Star Trek | Scryfall ID: b7285986-7e08-4969-86ef-452dc5bfdd9f | Oracle ID: f1750962-a87c-49f6-b731-02ae971ac6ea
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(65),
    oracle_id: "f1750962-a87c-49f6-b731-02ae971ac6ea",
    scryfall_id: "b7285986-7e08-4969-86ef-452dc5bfdd9f",
    faces: &[FaceDef {
        name: "Hallowed Fountain",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::land::PLAINS, subtypes::land::ISLAND],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::White]),
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
