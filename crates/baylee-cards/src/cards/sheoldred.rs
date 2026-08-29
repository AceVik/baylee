//! Sheoldred // The True Scriptures — {3}{B}{B} — Legendary Creature — Phyrexian Praetor // Enchantment — Saga
//! Oracle: Menace. When Sheoldred enters, each opponent sacrifices a nontoken creature or planeswalker of their choice. {4}{B}: Exile Sheoldred, then return it to the battlefield transformed under its owner's control. Activate only as a sorcery and only if an opponent has eight or more cards in their graveyard.
//! Oracle: The True Scriptures — I: For each opponent, destroy up to one target creature or planeswalker that player controls. II: Each opponent discards three cards, then mills three cards. III: Put all creature cards from all graveyards onto the battlefield under your control. Exile this Saga, then return it to the battlefield (front face up).
//! Set: MOM #125 — March of the Machine | Scryfall ID: bf2249e6-af74-4b88-8eb7-144ce8fa7f6b | Oracle ID: 97652492-7906-4d79-983c-fa1dc1239eba
// IMPLEMENTED — menace + ETB edict + conditional flip; all three saga
// chapters on the back face (lore counters, chapter triggers, sacrifice
// after III).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationCondition, ActivationTiming, ActivationZone, Amount, CardDef,
    CommanderRule, Cost, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind, PlayerRel,
    TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature, enchantment};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static NONTOKEN_CREATURE_OR_WALKER: Filter = Filter::And(&[
    Filter::Not(&Filter::IsToken),
    Filter::Or(&[
        Filter::HasType(TypeSet::CREATURE),
        Filter::HasType(TypeSet::PLANESWALKER),
    ]),
]);
static CREATURE_OR_WALKER: Filter = Filter::Or(&[
    Filter::HasType(TypeSet::CREATURE),
    Filter::HasType(TypeSet::PLANESWALKER),
]);

static BACK_ABILITIES: &[AbilityDef] = &[
    AbilityDef::SagaChapter {
        chapter: 1,
        effects: &[Effect::DestroyChosenForPlayers {
            who: PlayerRel::EachOpponent,
            filter: &CREATURE_OR_WALKER,
        }],
        target: None,
    },
    AbilityDef::SagaChapter {
        chapter: 2,
        effects: &[
            Effect::DiscardForPlayers {
                who: PlayerRel::EachOpponent,
                count: 3,
            },
            Effect::Mill {
                amount: Amount::Fixed(3),
                target: PlayerRel::EachOpponent,
            },
        ],
        target: None,
    },
    AbilityDef::SagaChapter {
        chapter: 3,
        effects: &[
            Effect::AllGraveyardCreaturesToBattlefield,
            Effect::ExileSelfReturnAsFace { face: 0 },
        ],
        target: None,
    },
];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(143),
    oracle_id: "97652492-7906-4d79-983c-fa1dc1239eba",
    scryfall_id: "bf2249e6-af74-4b88-8eb7-144ce8fa7f6b",
    faces: &[
        FaceDef {
            name: "Sheoldred",
            mana_cost: baylee_core::mana!("{3}{B}{B}"),
            types: TypeSet::CREATURE,
            supertypes: SupertypeSet::LEGENDARY,
            subtypes: &[creature::PHYREXIAN, creature::PRAETOR],
            power: Some(4),
            toughness: Some(5),
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
        },
        FaceDef {
            name: "The True Scriptures",
            mana_cost: baylee_core::mana!("{2}{B}{B}"),
            types: TypeSet::ENCHANTMENT,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[enchantment::SAGA],
            power: None,
            toughness: None,
            loyalty: None,
            alternative_costs: &[],
            additional_costs: &[],
            mandatory_additional_costs: &[],
            enter_modifiers: &[],
            abilities: BACK_ABILITIES,
            castable_from_hand: true,
            miracle: None,
            delve: false,
            convoke: false,
            cost_reduction: None,
            disturb: false,
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::MENACE,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefield(&Filter::This),
            once_per_turn: false,
            effects: &[Effect::SacrificeFilter {
                who: PlayerRel::EachOpponent,
                filter: &NONTOKEN_CREATURE_OR_WALKER,
            }],
            targets: None,
        },
        AbilityDef::ActivatedConditional {
            cost: Cost {
                mana: baylee_core::mana!("{4}{B}"),
                parts: &[],
            },
            effects: &[Effect::ExileSelfReturnAsFace { face: 1 }],
            target: None,
            timing: ActivationTiming::SorcerySpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
            condition: ActivationCondition::OpponentGraveyardCountAtLeast(8),
        },
    ],
};

#[cfg(test)]
mod tests {}
