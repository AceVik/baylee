//! Umara Raptor — {2}{U} — Creature — Bird Ally
//! Oracle: Flying
//! Oracle: Whenever this creature or another Ally you control enters, you may put a +1/+1 counter on this creature.
//! Set: ZEN #75 — Zendikar | Scryfall ID: 6049cc80-1faa-48bf-897e-fefe5a8e7ab2 | Oracle ID: a58ee84f-1d9c-4924-b7b1-14a9b2ba3b98
// IMPLEMENTED — flying + rally counter on self.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, CounterKind, Coverage, Effect, FaceDef, Filter,
    KeywordSet, PartnerKind, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ALLY_ETB: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::Or(&[Filter::This, Filter::HasSubtype(creature::ALLY)]),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(177),
    oracle_id: "a58ee84f-1d9c-4924-b7b1-14a9b2ba3b98",
    scryfall_id: "6049cc80-1faa-48bf-897e-fefe5a8e7ab2",
    faces: &[FaceDef {
        name: "Umara Raptor",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::BIRD, creature::ALLY],
        power: Some(1),
        toughness: Some(1),
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
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::FLYING,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&ALLY_ETB),
        once_per_turn: false,
        effects: &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
        targets: None,
    }],
};

#[cfg(test)]
mod tests {}
