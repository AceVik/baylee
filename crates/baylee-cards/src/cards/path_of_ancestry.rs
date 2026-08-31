//! Path of Ancestry — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add one mana of any color in your commander's color identity. When that mana is spent to cast a creature spell that shares a creature type with your commander, scry 1. (Look at the top card of your library. You may put that card on the bottom.)
//! Set: MBC #80 — Mystery Booster Commander Edition | Scryfall ID: b1aaa7b0-1cac-4a92-b880-7ef1ac00618f | Oracle ID: b473e293-59e3-4e04-acf2-622604aeb25f
// IMPLEMENTED — enters tapped + commander-identity mana with the scry
// rider: restricted mana that scries 1 when spent on a creature spell
// sharing a type with your commander.
// NOTE: the scry trigger queues via the synthetic-ability path (stacked
// as an ability).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Effect,
    EnterModifier, FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static COMMANDER_TYPE_CREATURE_SPELL: Filter = Filter::And(&[
    Filter::HasType(TypeSet::CREATURE),
    Filter::SharesSubtypeWithCommander,
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(111),
    oracle_id: "b473e293-59e3-4e04-acf2-622604aeb25f",
    scryfall_id: "b1aaa7b0-1cac-4a92-b880-7ef1ac00618f",
    faces: &[FaceDef {
        name: "Path of Ancestry",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::Tapped],
        ..FaceDef::DEFAULT
    }],
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddManaRestrictedCommanderIdentity {
            filter: &COMMANDER_TYPE_CREATURE_SPELL,
            rider: baylee_cards_dsl::SpendRider::Scry(1),
        }],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
    }],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
