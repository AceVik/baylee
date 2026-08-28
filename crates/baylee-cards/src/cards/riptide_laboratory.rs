//! Riptide Laboratory — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}{U}, {T}: Return target Wizard you control to its owner's hand.
//! Set: MH2 #303 — Modern Horizons 2 | Scryfall ID: 25a9cb87-e572-4885-8561-1d4b158ec7e4 | Oracle ID: 444d50dd-a44a-42db-bbf6-d0978e3bd6a3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(134),
    oracle_id: "444d50dd-a44a-42db-bbf6-d0978e3bd6a3",
    scryfall_id: "25a9cb87-e572-4885-8561-1d4b158ec7e4",
    faces: &[FaceDef {
        name: "Riptide Laboratory",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
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
