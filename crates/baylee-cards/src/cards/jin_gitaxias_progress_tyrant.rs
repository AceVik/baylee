//! Jin-Gitaxias, Progress Tyrant — {5}{U}{U} — Legendary Creature — Phyrexian Praetor
//! Oracle: Whenever you cast an artifact, instant, or sorcery spell, copy that spell. You may choose new targets for the copy. This ability triggers only once each turn. (A copy of a permanent spell becomes a token.)
//! Oracle: Whenever an opponent casts an artifact, instant, or sorcery spell, counter that spell. This ability triggers only once each turn.
//! Set: NEO #59 — Kamigawa: Neon Dynasty | Scryfall ID: c57b4876-5387-4f73-b8e2-8e7bdca8b0bc | Oracle ID: f5daadc1-98ff-480a-82bb-fe7bfaa7b60e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(78),
    oracle_id: "f5daadc1-98ff-480a-82bb-fe7bfaa7b60e",
    scryfall_id: "c57b4876-5387-4f73-b8e2-8e7bdca8b0bc",
    faces: &[FaceDef {
        name: "Jin-Gitaxias, Progress Tyrant",
        mana_cost: baylee_core::mana!("{5}{U}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::creature::PHYREXIAN, subtypes::creature::PRAETOR],
        power: Some(5),
        toughness: Some(5),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
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
