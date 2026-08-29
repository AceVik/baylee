//! Storm of Saruman — {4}{U}{U} — Enchantment
//! Oracle: Ward {3}
//! Oracle: Whenever you cast your second spell each turn, copy it, except the copy isn't legendary. You may choose new targets for the copy. (A copy of a permanent spell becomes a token.)
//! Set: LTR #72 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: 52884e67-c742-4799-9afd-55bc70b2cf40 | Oracle ID: cf5f4860-e805-46a3-9352-a2c583e33403
// PARTIAL — ward {3} and the second-spell copy trigger work; the
// "copy isn't legendary" mod and new-target choice for the copy are
// copy-machinery refinements (protocol M3).
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
        delve: false,
        convoke: false,
        cost_reduction: None,
        disturb: false,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("copy isn't legendary + copy target re-choice (protocol M3)"),
    abilities: &[
        AbilityDef::Ward { mana: 3 },
        AbilityDef::Triggered {
            trigger: Trigger::NthSpellCast {
                n: 2,
                filter: &YOUR_SPELL,
            },
            once_per_turn: false,
            effects: &[Effect::CopyTargetSpell],
            targets: Some(TargetReq::one(TargetSpec::EventObject)),
        },
    ],
};

#[cfg(test)]
mod tests {}
