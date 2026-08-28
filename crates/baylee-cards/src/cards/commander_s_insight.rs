//! Commander's Insight — {X}{U}{U}{U} — Instant
//! Oracle: Target player draws X cards plus an additional card for each time they've cast a commander from the command zone this game.
//! Set: SOC #113 — Secrets of Strixhaven Commander | Scryfall ID: 1a40e4da-a631-4423-b70f-701b27b09f79 | Oracle ID: 54d7d7f8-22cd-4859-b203-924d248b422b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(24),
    oracle_id: "54d7d7f8-22cd-4859-b203-924d248b422b",
    scryfall_id: "1a40e4da-a631-4423-b70f-701b27b09f79",
    faces: &[FaceDef {
        name: "Commander's Insight",
        mana_cost: baylee_core::mana!("{X}{U}{U}{U}"),
        types: TypeSet::INSTANT,
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
