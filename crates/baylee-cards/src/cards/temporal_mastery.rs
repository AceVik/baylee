//! Temporal Mastery — {5}{U}{U} — Sorcery
//! Oracle: Take an extra turn after this one. Exile Temporal Mastery.
//! Oracle: Miracle {1}{U} (You may cast this card for its miracle cost when you draw it if it's the first card you drew this turn.)
//! Set: INR #90 — Innistrad Remastered | Scryfall ID: 0f46a800-b443-461d-87e0-5587249a42d8 | Oracle ID: 5c58b8e6-c572-461e-893e-a8c05f20ba17
// IMPLEMENTED — extra-turn queue + self-exile + miracle cast.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(168),
    oracle_id: "5c58b8e6-c572-461e-893e-a8c05f20ba17",
    scryfall_id: "0f46a800-b443-461d-87e0-5587249a42d8",
    faces: &[FaceDef {
        name: "Temporal Mastery",
        mana_cost: baylee_core::mana!("{5}{U}{U}"),
        types: TypeSet::SORCERY,
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
        miracle: Some(baylee_core::mana!("{1}{U}")),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::TakeExtraTurn, Effect::ExileSource],
        targets: None,
    }],
};

#[cfg(test)]
mod tests {}
