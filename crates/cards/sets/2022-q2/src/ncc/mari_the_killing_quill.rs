//! Mari, the Killing Quill — {1}{B}{B} — Legendary Creature — Vampire Assassin
//! Oracle: Whenever a creature an opponent controls dies, exile it with a hit counter on it.
//! Oracle: Assassins, Mercenaries, and Rogues you control have deathtouch and "Whenever this creature deals combat damage to a player, you may remove a hit counter from a card that player owns in exile. If you do, draw a card and create two Treasure tokens."
//! Set: OTC #138 — Outlaws of Thunder Junction Commander | Scryfall ID: 6b4b3c1a-671c-422d-a98c-7d750760ca46 | Oracle ID: 5f1fdc23-9af0-41a1-aeba-7288f9642734
// IMPLEMENTED

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static MARI_ALLIES: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::Or(&[
        Filter::HasSubtype(subtypes::creature::ASSASSIN),
        Filter::HasSubtype(subtypes::creature::MERCENARY),
        Filter::HasSubtype(subtypes::creature::ROGUE),
    ]),
]);

card!(
    index = index::MARI_THE_KILLING_QUILL,
    oracle_id = "5f1fdc23-9af0-41a1-aeba-7288f9642734",
    scryfall_id = "6b4b3c1a-671c-422d-a98c-7d750760ca46",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mari, the Killing Quill",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::VAMPIRE, subtypes::creature::ASSASSIN],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[
        triggered!(
            Trigger::Dies(&Filter::OPPONENT_CREATURE),
            &[Effect::ExileEventFromGraveyardWithCounters {
                kind: counters::HIT,
                n: 1,
            }]
        ),
        static_ability!(MARI_ALLIES, Modifier::AddKeyword(KeywordSet::DEATHTOUCH)),
        static_ability!(
            MARI_ALLIES,
            Modifier::GrantTriggered {
                trigger: Trigger::DealsCombatDamageToPlayer(&Filter::This),
                effects: &[Effect::MayDo {
                    effects: &[
                        Effect::RemoveCountersFromChosenExile {
                            owner: PlayerRel::EventPlayer,
                            kind: counters::HIT,
                            n: 1,
                        },
                        Effect::draw(1),
                        Effect::CreateTokenN {
                            token: &generated_tokens::TREASURE,
                            amount: Amount::Fixed(2),
                        },
                    ],
                }],
                target: None,
            }
        ),
    ],
);
