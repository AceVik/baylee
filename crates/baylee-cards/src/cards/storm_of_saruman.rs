//! Storm of Saruman — {4}{U}{U} — Enchantment
//! Oracle: Ward {3}
//! Oracle: Whenever you cast your second spell each turn, copy it, except the copy isn't legendary. You may choose new targets for the copy. (A copy of a permanent spell becomes a token.)
//! Set: LTR #72 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: 52884e67-c742-4799-9afd-55bc70b2cf40 | Oracle ID: cf5f4860-e805-46a3-9352-a2c583e33403
// IMPLEMENTED — ward {3}, the second-spell copy carrying the
// non-legendary mod, and the copy's new-target choice.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static YOUR_SPELL: Filter = Filter::ControlledByYou;

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(157),
    oracle_id: "cf5f4860-e805-46a3-9352-a2c583e33403",
    scryfall_id: "52884e67-c742-4799-9afd-55bc70b2cf40",
    faces: &[FaceDef {
        name: "Storm of Saruman",
        mana_cost: baylee_core::mana!("{4}{U}{U}"),
        types: TypeSet::ENCHANTMENT,
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Ward { mana: 3 },
        AbilityDef::Triggered {
            trigger: Trigger::NthSpellCast {
                n: 2,
                filter: &YOUR_SPELL,
            },
            once_per_turn: false,
            effects: &[Effect::CopyTargetSpell {
                mods: &[baylee_cards_dsl::CopyMod::RemoveSupertype(
                    SupertypeSet::LEGENDARY,
                )],
            }],
            targets: Some(TargetReq::one(TargetSpec::EventObject)),
        },
    ],
    ..CardDef::DEFAULT
};
