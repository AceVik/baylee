//! Gilded Drake — {1}{U} — Creature — Drake
//! Oracle: Flying
//! Oracle: When this creature enters, exchange control of this creature and up to one target creature an opponent controls. If you don't or can't make an exchange, sacrifice this creature. This ability still resolves if its target becomes illegal.
//! Set: USG #76 — Urza's Saga | Scryfall ID: 8de3fdae-cc2c-4a14-b15b-4fe1a983dfbf | Oracle ID: 7f06c098-6482-4bf3-a9a1-110d6d5b5703
// IMPLEMENTED — control exchange with sacrifice fallback.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static OPPONENT_CREATURE: Filter = Filter::And(&[
    Filter::HasType(TypeSet::CREATURE),
    Filter::ControlledByOpponent,
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(58),
    oracle_id: "7f06c098-6482-4bf3-a9a1-110d6d5b5703",
    scryfall_id: "8de3fdae-cc2c-4a14-b15b-4fe1a983dfbf",
    faces: &[FaceDef {
        name: "Gilded Drake",
        mana_cost: baylee_core::mana!("{1}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::DRAKE],
        power: Some(2),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::FLYING,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        once_per_turn: false,
        effects: &[Effect::ExchangeControlOrSacrifice],
        targets: Some(TargetReq {
            spec: TargetSpec::Object(&OPPONENT_CREATURE),
            min: 0,
            max: 1,
            count_is_x: false,
        }),
    }],
};

#[cfg(test)]
mod tests {}
