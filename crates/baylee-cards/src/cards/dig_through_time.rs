//! Dig Through Time — {6}{U}{U} — Instant
//! Oracle: Delve (Each card you exile from your graveyard while casting this spell pays for {1}.)
//! Oracle: Look at the top seven cards of your library. Put two of them into your hand and the rest on the bottom of your library in any order.
//! Set: SOC #195 — Secrets of Strixhaven Commander | Scryfall ID: 020939d6-72f0-4aa0-9ac2-d16cc896cd7f | Oracle ID: f8b17b89-26ce-4208-874a-9e1d66514640
// IMPLEMENTED — delve cost reduction + look-7-pick-2.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(33),
    oracle_id: "f8b17b89-26ce-4208-874a-9e1d66514640",
    scryfall_id: "020939d6-72f0-4aa0-9ac2-d16cc896cd7f",
    faces: &[FaceDef {
        name: "Dig Through Time",
        mana_cost: baylee_core::mana!("{6}{U}{U}"),
        types: TypeSet::INSTANT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
        delve: true,
        convoke: false,
        cost_reduction: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::LookAtTopPick { count: 7, pick: 2 }],
        targets: None,
    }],
};

#[cfg(test)]
mod tests {}
