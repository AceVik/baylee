//! Sheoldred // The True Scriptures — (no cost) — Legendary Creature — Phyrexian Praetor // Enchantment — Saga
//! Set: MOM #125 — March of the Machine | Scryfall ID: bf2249e6-af74-4b88-8eb7-144ce8fa7f6b | Oracle ID: 97652492-7906-4d79-983c-fa1dc1239eba
//! Face: Sheoldred — {3}{B}{B} — Legendary Creature — Phyrexian Praetor
//! Face: The True Scriptures —  — Enchantment — Saga
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(143),
    oracle_id: "97652492-7906-4d79-983c-fa1dc1239eba",
    scryfall_id: "bf2249e6-af74-4b88-8eb7-144ce8fa7f6b",
    faces: &[
        FaceDef {
            name: "Sheoldred",
            mana_cost: baylee_core::mana!("{3}{B}{B}"),
            types: TypeSet::CREATURE,
            supertypes: SupertypeSet::LEGENDARY,
            subtypes: &[subtypes::creature::PHYREXIAN, subtypes::creature::PRAETOR],
            power: Some(4),
            toughness: Some(5),
            loyalty: None,
            alternative_costs: &[],
            additional_costs: &[],
            mandatory_additional_costs: &[],
        enter_modifiers: &[],
        },
        FaceDef {
            name: "The True Scriptures",
            mana_cost: ManaCost::ZERO,
            types: TypeSet::ENCHANTMENT,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[subtypes::enchantment::SAGA],
            power: None,
            toughness: None,
            loyalty: None,
            alternative_costs: &[],
            additional_costs: &[],
            mandatory_additional_costs: &[],
        enter_modifiers: &[],
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Unimplemented,
    abilities: &[],
};

#[cfg(test)]
mod tests {
    // TODO(card): implement abilities + tests, see docs/card-dsl.md.
}
