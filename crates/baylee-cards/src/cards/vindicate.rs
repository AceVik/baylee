//! Vindicate — {1}{W}{B} — Sorcery
//! Oracle: Destroy target permanent.
//! Set: MH2 #294 — Modern Horizons 2 | Scryfall ID: 683c4e13-525c-45c9-8832-bfe67965c34e | Oracle ID: 63c1ac21-e3d8-40c2-8c09-3f31c52992ef
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(184),
    oracle_id: "63c1ac21-e3d8-40c2-8c09-3f31c52992ef",
    scryfall_id: "683c4e13-525c-45c9-8832-bfe67965c34e",
    faces: &[FaceDef {
        name: "Vindicate",
        mana_cost: baylee_core::mana!("{1}{W}{B}"),
        types: TypeSet::SORCERY,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
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
