//! Orcish Bowmasters — {1}{B} — Creature — Orc Archer
//! Oracle: Flash
//! Oracle: When this creature enters, it deals 1 damage to target opponent. Amass Orcs 1.
//! Oracle: Whenever an opponent draws a card except the first one they draw each turn, amass Orcs 1.
//! Set: LTR #103 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: 7c024bae-5631-4e20-ac69-df392ac9e109 | Oracle ID: ea5103f5-27e0-4eb1-902c-7f34652d6bf3
// IMPLEMENTED — flash + ping + amass on opponents' extra draws.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, PlayerRel, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(106),
    oracle_id: "ea5103f5-27e0-4eb1-902c-7f34652d6bf3",
    scryfall_id: "7c024bae-5631-4e20-ac69-df392ac9e109",
    faces: &[FaceDef {
        name: "Orcish Bowmasters",
        mana_cost: baylee_core::mana!("{1}{B}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::ORC, creature::ARCHER],
        power: Some(1),
        toughness: Some(1),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::FLASH,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefield(&Filter::This),
            once_per_turn: false,
            effects: &[
                Effect::DealDamage {
                    amount: Amount::Fixed(1),
                    target: TargetSpec::Player(PlayerRel::Opponent),
                },
                Effect::Amass {
                    token: &crate::tokens::ARMY_0_0_BLACK,
                    subtype: creature::ORC,
                    amount: 1,
                },
            ],
            targets: None,
        },
        AbilityDef::Triggered {
            trigger: Trigger::DrawsExceptFirst(PlayerRel::Opponent),
            once_per_turn: false,
            effects: &[Effect::Amass {
                token: &crate::tokens::ARMY_0_0_BLACK,
                subtype: creature::ORC,
                amount: 1,
            }],
            targets: None,
        },
    ],
    ..CardDef::DEFAULT
};
