//! Stangg — {4}{R}{G} — Legendary Creature — Human Warrior
//! Oracle: When Stangg enters, create Stangg Twin, a legendary 3/4 red and green Human Warrior creature token. Exile that token when Stangg leaves the battlefield. Sacrifice Stangg when that token leaves the battlefield.
//! Set: A25 #218 — Masters 25 | Scryfall ID: a23f846e-b5e6-48c2-afea-c9be7fc8b7c3 | Oracle ID: 739eac91-3029-4ce7-9885-0af3ddea472e

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STANGG,
    oracle_id = "739eac91-3029-4ce7-9885-0af3ddea472e",
    scryfall_id = "a23f846e-b5e6-48c2-afea-c9be7fc8b7c3",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Stangg",
        mana_cost = mana!("{4}{R}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARRIOR],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[
            Effect::CreateToken {
                token: &generated_tokens::STANGG_TWIN_LEGENDARY_3_4_RED_GREEN,
            },
            Effect::WhenSourceLeavesBattlefieldForCreatedTokens {
                effects: &[Effect::Exile {
                    target: TargetSpec::EventObject,
                }],
            },
            Effect::WhenCreatedTokenLeavesBattlefield {
                effects: &[Effect::SacrificeSelf],
            },
        ]
    )],
);
