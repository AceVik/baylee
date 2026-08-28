//! Ravenous Chupacabra — {2}{B}{B} — Creature — Beast Horror
//! Oracle: When this creature enters, destroy target creature an opponent controls.
//! Set: MKC #136 — Murders at Karlov Manor Commander | Scryfall ID: a4dfbac0-1849-41c5-853a-1fee108d0b01 | Oracle ID: 7b459306-149b-4f43-abc1-2dd70c748c0e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(124),
    oracle_id: "7b459306-149b-4f43-abc1-2dd70c748c0e",
    scryfall_id: "a4dfbac0-1849-41c5-853a-1fee108d0b01",
    faces: &[FaceDef {
        name: "Ravenous Chupacabra",
        mana_cost: baylee_core::mana!("{2}{B}{B}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::BEAST, subtypes::creature::HORROR],
        power: Some(2),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
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
