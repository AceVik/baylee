//! Earth King's Lieutenant — {3}{G} — Creature — Human Soldier Ally
//! Oracle: Trample
//! Oracle: When this creature enters, put a +1/+1 counter on each other Ally creature you control.
//! Oracle: Whenever another Ally you control enters, put a +1/+1 counter on this creature.
//! Set: TLA #174 — Avatar: The Last Airbender | Scryfall ID: 4533d155-5c56-41a5-9d76-2d1414ac47c9 | Oracle ID: 9da9248d-1201-447f-b6c2-2b64af4f71c4
// IMPLEMENTED — trample + ETB team counters + rally counter on self.
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

static OTHER_ALLIES: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasSubtype(creature::ALLY),
    Filter::Another,
]);
static ALLY_ETB: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasSubtype(creature::ALLY),
    Filter::Another,
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(37),
    oracle_id: "9da9248d-1201-447f-b6c2-2b64af4f71c4",
    scryfall_id: "4533d155-5c56-41a5-9d76-2d1414ac47c9",
    faces: &[FaceDef {
        name: "Earth King's Lieutenant",
        mana_cost: baylee_core::mana!("{3}{G}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::HUMAN, creature::SOLDIER, creature::ALLY],
        power: Some(3),
        toughness: Some(3),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Green]),
    keywords: KeywordSet::TRAMPLE,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefield(&Filter::This),
            once_per_turn: false,
            effects: &[Effect::AddCounterFilter {
                filter: &OTHER_ALLIES,
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
            targets: None,
        },
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefield(&ALLY_ETB),
            once_per_turn: false,
            effects: &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
            targets: None,
        },
    ],
};

#[cfg(test)]
mod tests {}
