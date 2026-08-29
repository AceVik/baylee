//! Mana Drain — {U}{U} — Instant
//! Oracle: Counter target spell. At the beginning of your next main phase, add an amount of {C} equal to that spell's mana value.
//! Set: 2X2 #57 — Double Masters 2022 | Scryfall ID: 3c429c40-2389-41e5-8681-4bb274e25eba | Oracle ID: 74d3277a-38e5-4732-afed-084a56148f20
// IMPLEMENTED — counter + delayed colorless mana at your next first main.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_SPELL: Filter = Filter::Any;

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(90),
    oracle_id: "74d3277a-38e5-4732-afed-084a56148f20",
    scryfall_id: "3c429c40-2389-41e5-8681-4bb274e25eba",
    faces: &[FaceDef {
        name: "Mana Drain",
        mana_cost: baylee_core::mana!("{U}{U}"),
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
        delve: false,
        convoke: false,
        cost_reduction: None,
        disturb: false,
        adventure: false,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[
            Effect::CounterTargetSpell,
            Effect::DelayedManaAtNextFirstMain {
                color: ManaColor::Colorless,
            },
        ],
        targets: Some(TargetReq::one(TargetSpec::Spell(&ANY_SPELL))),
    }],
};

#[cfg(test)]
mod tests {}
