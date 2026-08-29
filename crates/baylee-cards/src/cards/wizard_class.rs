//! Wizard Class — {U} — Enchantment — Class
//! Oracle: (Gain the next level as a sorcery to add its ability.)
//! Oracle: You have no maximum hand size.
//! Oracle: {2}{U}: Level 2 — When this Class becomes level 2, draw two cards.
//! Oracle: {4}{U}: Level 3 — Whenever you draw a card, put a +1/+1 counter on target creature you control.
//! Set: AFR #81 — Adventures in the Forgotten Realms | Scryfall ID: d1f629fb-b097-4240-8560-ef47f5678f48 | Oracle ID: 36f68aa3-9955-46f1-bc87-497f16ef5222
// PARTIAL — level 1 (no max hand size via hand modifier... engine's hand
// modifier is static; the class level system needs Class-level tracking
// (M2+). Everything else needs that same system.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(192),
    oracle_id: "36f68aa3-9955-46f1-bc87-497f16ef5222",
    scryfall_id: "d1f629fb-b097-4240-8560-ef47f5678f48",
    faces: &[FaceDef {
        name: "Wizard Class",
        mana_cost: baylee_core::mana!("{U}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("Class level system (M2+)"),
    abilities: &[],
};

#[cfg(test)]
mod tests {}
