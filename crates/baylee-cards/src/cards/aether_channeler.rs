//! Aether Channeler — {2}{U} — Creature — Human Wizard
//! Oracle: When this creature enters, choose one —
//! Oracle: • Create a 1/1 white Bird creature token with flying.
//! Oracle: • Return another target nonland permanent to its owner's hand.
//! Oracle: • Draw a card.
//! Set: DMU #42 — Dominaria United | Scryfall ID: 60afeb75-2c1e-4634-8c83-88b1dddb77c2 | Oracle ID: fb220f46-f8b8-4804-baa4-e7d50b4871f7
// IMPLEMENTED — modal ETB with all three modes.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, SpellMode, TargetSpec, TokenDef, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

use crate::tokens::BIRD_1_1_WHITE_FLYING as BIRD_TOKEN;
static TOKEN_EFFECTS: &[Effect] = &[Effect::CreateToken { token: &BIRD_TOKEN }];
static BOUNCE_TARGET: Filter = Filter::And(&[Filter::LacksType(TypeSet::LAND), Filter::Another]);
static BOUNCE_EFFECTS: &[Effect] = &[Effect::ReturnToHand {
    target: TargetSpec::Object(&BOUNCE_TARGET),
}];
static DRAW_EFFECTS: &[Effect] = &[Effect::DrawCards {
    amount: Amount::Fixed(1),
}];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(2),
    oracle_id: "fb220f46-f8b8-4804-baa4-e7d50b4871f7",
    scryfall_id: "60afeb75-2c1e-4634-8c83-88b1dddb77c2",
    faces: &[FaceDef {
        name: "Aether Channeler",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::HUMAN, creature::WIZARD],
        power: Some(2),
        toughness: Some(1),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::ModalTriggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        modes: &[
            SpellMode {
                effects: TOKEN_EFFECTS,
                target: None,
                cost_override: None,
            },
            SpellMode {
                effects: BOUNCE_EFFECTS,
                target: Some(TargetSpec::Object(&BOUNCE_TARGET)),
                cost_override: None,
            },
            SpellMode {
                effects: DRAW_EFFECTS,
                target: None,
                cost_override: None,
            },
        ],
        once_per_turn: false,
    }],
    ..CardDef::DEFAULT
};
