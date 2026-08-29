//! Tribute to the World Tree — {G}{G}{G} — Enchantment
//! Oracle: Whenever a creature you control enters, draw a card if its power is 3 or greater. Otherwise, put two +1/+1 counters on it.
//! Set: MOM #211 — March of the Machine | Scryfall ID: c0cdeaba-fc21-44e6-bf99-aa1ff379401b | Oracle ID: 72deedab-7c17-4505-aeca-4bc8596d80a5
// IMPLEMENTED — power-conditional ETB trigger (draw or two counters).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, CounterKind, Coverage, Effect, FaceDef, Filter,
    KeywordSet, PartnerKind, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static YOUR_CREATURE: Filter =
    Filter::And(&[Filter::HasType(TypeSet::CREATURE), Filter::ControlledByYou]);
static THEN_DRAW: &[Effect] = &[Effect::DrawCards {
    amount: Amount::Fixed(1),
}];
// The nested resolution targets the event object (the entering creature).
static ELSE_COUNTERS: &[Effect] = &[Effect::AddCounter {
    kind: CounterKind::P1P1,
    amount: Amount::Fixed(2),
}];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(173),
    oracle_id: "72deedab-7c17-4505-aeca-4bc8596d80a5",
    scryfall_id: "c0cdeaba-fc21-44e6-bf99-aa1ff379401b",
    faces: &[FaceDef {
        name: "Tribute to the World Tree",
        mana_cost: baylee_core::mana!("{G}{G}{G}"),
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
        adventure: false,
    }],
    color_identity: ColorSet::from_slice(&[Color::Green]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&YOUR_CREATURE),
        once_per_turn: false,
        effects: &[Effect::IfEventPowerAtLeast {
            n: 3,
            then: THEN_DRAW,
            otherwise: ELSE_COUNTERS,
        }],
        targets: None,
    }],
};

#[cfg(test)]
mod tests {}
