//! Sheoldred // The True Scriptures — {3}{B}{B} — Legendary Creature — Phyrexian Praetor // Enchantment — Saga
//! Oracle: Menace. When Sheoldred enters, each opponent sacrifices a nontoken creature or planeswalker of their choice. {4}{B}: Exile Sheoldred, then return it to the battlefield transformed under its owner's control. Activate only as a sorcery and only if an opponent has eight or more cards in their graveyard.
//! Oracle: The True Scriptures — I: For each opponent, destroy up to one target creature or planeswalker that player controls. II: Each opponent discards three cards, then mills three cards. III: Put all creature cards from all graveyards onto the battlefield under your control. Exile this Saga, then return it to the battlefield (front face up).
//! Set: MOM #125 — March of the Machine | Scryfall ID: bf2249e6-af74-4b88-8eb7-144ce8fa7f6b | Oracle ID: 97652492-7906-4d79-983c-fa1dc1239eba
// IMPLEMENTED — menace + ETB edict + conditional flip; all three saga
// chapters on the back face (lore counters, chapter triggers, sacrifice
// after III).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{creature, enchantment};

static NONTOKEN_CREATURE_OR_WALKER: Filter = Filter::And(&[
    Filter::Not(&Filter::IsToken),
    Filter::Or(&[Filter::CREATURE, Filter::PLANESWALKER]),
]);
static CREATURE_OR_WALKER: Filter = Filter::Or(&[Filter::CREATURE, Filter::PLANESWALKER]);

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

card! {
    index: 143,
    oracle_id: "97652492-7906-4d79-983c-fa1dc1239eba",
    scryfall_id: "bf2249e6-af74-4b88-8eb7-144ce8fa7f6b",
    faces: &[
        face! {
            name: "Sheoldred",
            mana_cost: baylee_core::mana!("{3}{B}{B}"),
            types: TypeSet::CREATURE,
            supertypes: SupertypeSet::LEGENDARY,
            subtypes: &[creature::PHYREXIAN, creature::PRAETOR],
            power: Some(4),
            toughness: Some(5),
        },
        face! {
            name: "The True Scriptures",
            mana_cost: baylee_core::mana!("{2}{B}{B}"),
            types: TypeSet::ENCHANTMENT,
            subtypes: &[enchantment::SAGA],
            abilities: BACK_ABILITIES,
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::MENACE,
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::SacrificeFilter {
                who: PlayerRel::EachOpponent,
                filter: &NONTOKEN_CREATURE_OR_WALKER,
            }]),
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
}
