//! Ertai Resurrected — {2}{U}{B} — Legendary Creature — Phyrexian Human Wizard
//! Oracle: Flash
//! Oracle: When Ertai Resurrected enters, choose up to one —
//! Oracle: • Counter target spell, activated ability, or triggered ability. Its controller draws a card.
//! Oracle: • Destroy another target creature or planeswalker. Its controller draws a card.
//! Set: DMU #199 — Dominaria United | Scryfall ID: 7f7e780e-fbc5-4dc0-b5c7-efcb8645c7c6 | Oracle ID: 3d038f7c-95fa-4b71-8f74-b9b4dd45cde0
// IMPLEMENTED — flash + modal ETB (counter / destroy / decline via the
// empty third mode for "up to one").
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, PlayerRel, SpellMode, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY: Filter = Filter::Any;
static OTHER_CREATURE_OR_WALKER: Filter = Filter::And(&[
    Filter::Another,
    Filter::Or(&[
        Filter::HasType(TypeSet::CREATURE),
        Filter::HasType(TypeSet::PLANESWALKER),
    ]),
]);
static COUNTER_EFFECTS: &[Effect] = &[
    Effect::CounterTargetSpellOrAbility,
    Effect::DrawCardsFor {
        amount: Amount::Fixed(1),
        who: PlayerRel::ControllerOfTarget,
    },
];
static DESTROY_EFFECTS: &[Effect] = &[
    Effect::Destroy {
        target: TargetSpec::Object(&OTHER_CREATURE_OR_WALKER),
    },
    Effect::DrawCardsFor {
        amount: Amount::Fixed(1),
        who: PlayerRel::ControllerOfTarget,
    },
];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(45),
    oracle_id: "3d038f7c-95fa-4b71-8f74-b9b4dd45cde0",
    scryfall_id: "7f7e780e-fbc5-4dc0-b5c7-efcb8645c7c6",
    faces: &[FaceDef {
        name: "Ertai Resurrected",
        mana_cost: baylee_core::mana!("{2}{U}{B}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[creature::PHYREXIAN, creature::HUMAN, creature::WIZARD],
        power: Some(3),
        toughness: Some(2),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Black]),
    keywords: KeywordSet::FLASH,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::ModalTriggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        modes: &[
            SpellMode {
                effects: COUNTER_EFFECTS,
                target: Some(TargetSpec::SpellOrAbility(&ANY)),
                cost_override: None,
            },
            SpellMode {
                effects: DESTROY_EFFECTS,
                target: Some(TargetSpec::Object(&OTHER_CREATURE_OR_WALKER)),
                cost_override: None,
            },
            // "Choose up to one" — declining is mode 2 (no effects).
            SpellMode {
                effects: &[],
                target: None,
                cost_override: None,
            },
        ],
        once_per_turn: false,
    }],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
