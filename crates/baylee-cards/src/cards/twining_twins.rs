//! Twining Twins // Swift Spiral — {2}{U} — Creature — Faerie Wizard // Instant — Adventure
//! Oracle: Flash. When Twining Twins enters, choose one —
//! Oracle: • This creature gains flying until end of turn.
//! Oracle: • Put a +1/+1 counter on target creature you control.
//! Set: EOC #66 — Edge of Eternities Commander | Scryfall ID: 043718ea-59f6-4d1a-94c5-271704c1a38a | Oracle ID: 105aea98-8eb9-4fb2-a0cb-7c7513317c5b
// PARTIAL — flash body + modal ETB implemented; the Adventure back face
// (Swift Spiral) needs Adventure casting (M2.S8+).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, CounterKind, Coverage, Duration, Effect, FaceDef,
    Filter, KeywordSet, Layer, Modifier, PartnerKind, SpellMode, TargetReq, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static FLY_EFFECTS: &[Effect] = &[Effect::CreateContinuousEffect {
    layer: Layer::Ability,
    filter: &Filter::This,
    modifier: Modifier::AddKeyword(KeywordSet::FLYING),
    duration: Duration::UntilEndOfTurn,
}];
static COUNTER_EFFECTS: &[Effect] = &[Effect::AddCounter {
    kind: CounterKind::P1P1,
    amount: Amount::Fixed(1),
}];
static YOUR_CREATURE: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::HasType(TypeSet::CREATURE)]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(176),
    oracle_id: "105aea98-8eb9-4fb2-a0cb-7c7513317c5b",
    scryfall_id: "043718ea-59f6-4d1a-94c5-271704c1a38a",
    faces: &[FaceDef {
        name: "Twining Twins",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::FAERIE, creature::WIZARD],
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
        delve: false,
        convoke: false,
        cost_reduction: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::FLASH,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("Adventure back face (M2.S8+)"),
    abilities: &[AbilityDef::ModalTriggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        modes: &[
            SpellMode {
                effects: FLY_EFFECTS,
                target: None,
                cost_override: None,
            },
            SpellMode {
                effects: COUNTER_EFFECTS,
                target: Some(TargetSpec::Object(&YOUR_CREATURE)),
                cost_override: None,
            },
        ],
        once_per_turn: false,
    }],
};

#[cfg(test)]
mod tests {}
