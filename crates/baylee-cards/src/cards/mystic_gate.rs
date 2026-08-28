//! Mystic Gate — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {W/U}, {T}: Add {W}{W}, {W}{U}, or {U}{U}.
//! Set: CMM #1013 — Commander Masters | Scryfall ID: 6f99714f-43bc-4048-b650-97dfef4c10fe | Oracle ID: e9f5feb2-2c1a-46ce-885a-4f378d7d10af
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(101),
    oracle_id: "e9f5feb2-2c1a-46ce-885a-4f378d7d10af",
    scryfall_id: "6f99714f-43bc-4048-b650-97dfef4c10fe",
    faces: &[FaceDef {
        name: "Mystic Gate",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
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
