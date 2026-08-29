//! Charming Prince — {1}{W} — Creature — Human Noble
//! Oracle: When this creature enters, choose one —
//! Oracle: • Scry 2.
//! Oracle: • You gain 3 life.
//! Oracle: • Exile another target creature you own. Return it to the battlefield under your control at the beginning of the next end step.
//! Set: TDS #8 — Tarkir: Dragonstorm | Scryfall ID: aa7b47e1-7e32-4f2f-aecf-bac7ca197081 | Oracle ID: c48d844c-3976-4fa5-8e0d-3f0e535e7619
// IMPLEMENTED — all three modes (scry, lifegain, end-step blink).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, SpellMode, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static SCRY_EFFECTS: &[Effect] = &[Effect::Scry {
    amount: Amount::Fixed(2),
}];
static LIFE_EFFECTS: &[Effect] = &[Effect::GainLife {
    amount: Amount::Fixed(3),
}];
static BLINK_EFFECTS: &[Effect] = &[Effect::ExileAndReturnAtEndStep];
static OTHER_CREATURE_YOU_OWN: Filter = Filter::And(&[
    Filter::Another,
    Filter::HasType(TypeSet::CREATURE),
    Filter::OwnedByYou,
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(18),
    oracle_id: "c48d844c-3976-4fa5-8e0d-3f0e535e7619",
    scryfall_id: "aa7b47e1-7e32-4f2f-aecf-bac7ca197081",
    faces: &[FaceDef {
        name: "Charming Prince",
        mana_cost: baylee_core::mana!("{1}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::HUMAN, creature::NOBLE],
        power: Some(2),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::ModalTriggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        modes: &[
            SpellMode {
                effects: SCRY_EFFECTS,
                target: None,
                cost_override: None,
            },
            SpellMode {
                effects: LIFE_EFFECTS,
                target: None,
                cost_override: None,
            },
            SpellMode {
                effects: BLINK_EFFECTS,
                target: Some(TargetSpec::Object(&OTHER_CREATURE_YOU_OWN)),
                cost_override: None,
            },
        ],
        once_per_turn: false,
    }],
};

#[cfg(test)]
mod tests {}
