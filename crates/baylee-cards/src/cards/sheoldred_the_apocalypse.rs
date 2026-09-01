//! Sheoldred, the Apocalypse — {2}{B}{B} — Legendary Creature — Phyrexian Praetor
//! Oracle: Deathtouch
//! Oracle: Whenever you draw a card, you gain 2 life.
//! Oracle: Whenever an opponent draws a card, they lose 2 life.
//! Set: DMU #107 — Dominaria United | Scryfall ID: d67be074-cdd4-41d9-ac89-0a0456c4e4b2 | Oracle ID: 34f34409-326d-4994-a0ea-1a69aa278f03
// IMPLEMENTED — deathtouch + draw-punish both directions.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, KeywordSet, PartnerKind,
    PlayerRel, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(145),
    oracle_id: "34f34409-326d-4994-a0ea-1a69aa278f03",
    scryfall_id: "d67be074-cdd4-41d9-ac89-0a0456c4e4b2",
    faces: &[FaceDef {
        name: "Sheoldred, the Apocalypse",
        mana_cost: baylee_core::mana!("{2}{B}{B}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[creature::PHYREXIAN, creature::PRAETOR],
        power: Some(4),
        toughness: Some(5),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::DEATHTOUCH,
    commander: CommanderRule::Legendary,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::Draws(PlayerRel::You),
            once_per_turn: false,
            effects: &[Effect::GainLife {
                amount: Amount::Fixed(2),
            }],
            targets: None,
        },
        AbilityDef::Triggered {
            trigger: Trigger::Draws(PlayerRel::Opponent),
            once_per_turn: false,
            effects: &[Effect::LoseLife {
                amount: Amount::Fixed(2),
                target: PlayerRel::Opponent,
            }],
            targets: None,
        },
    ],
    ..CardDef::DEFAULT
};
