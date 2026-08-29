//! Sheoldred // The True Scriptures — {3}{B}{B} — Legendary Creature — Phyrexian Praetor // Enchantment — Saga
//! Oracle: Menace. Whenever you draw a card, each opponent loses 2 life. Whenever an opponent draws a card, you gain 2 life. // (Saga with three chapters — see saga milestone.)
//! Set: MOM #125 — March of the Machine | Scryfall ID: bf2249e6-af74-4b88-8eb7-144ce8fa7f6b | Oracle ID: 97652492-7906-4d79-983c-fa1dc1239eba
// PARTIAL — Sheoldred (front) fully implemented (menace + both draw
// triggers). The True Scriptures is castable via the MDFC face choice
// but has no abilities until the saga milestone lands.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, PlayerRel, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature, enchantment};
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
            subtypes: &[creature::PHYREXIAN, creature::PRAETOR],
            power: Some(4),
            toughness: Some(6),
            loyalty: None,
            alternative_costs: &[],
            additional_costs: &[],
            mandatory_additional_costs: &[],
            enter_modifiers: &[],
            abilities: &[],
            castable_from_hand: true,
            miracle: None,
            delve: false,
            convoke: false,
            cost_reduction: None,
        },
        FaceDef {
            name: "The True Scriptures",
            mana_cost: baylee_core::mana!("{2}{B}{B}"),
            types: TypeSet::ENCHANTMENT,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[enchantment::SAGA],
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
            delve: false,
            convoke: false,
            cost_reduction: None,
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::MENACE,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("The True Scriptures saga chapters (saga milestone)"),
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::Draws(PlayerRel::You),
            once_per_turn: false,
            effects: &[Effect::LoseLife {
                amount: Amount::Fixed(2),
                target: PlayerRel::EachOpponent,
            }],
            targets: None,
        },
        AbilityDef::Triggered {
            trigger: Trigger::Draws(PlayerRel::Opponent),
            once_per_turn: false,
            effects: &[Effect::GainLife {
                amount: Amount::Fixed(2),
            }],
            targets: None,
        },
    ],
};

#[cfg(test)]
mod tests {}
