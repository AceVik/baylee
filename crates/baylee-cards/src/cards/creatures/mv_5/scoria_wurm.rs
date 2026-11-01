//! Scoria Wurm — {4}{R} — Creature — Wurm
//! Oracle: At the beginning of your upkeep, flip a coin. If you lose the flip, return this creature to its owner's hand.
//! Set: USG #212 — Urza's Saga | Scryfall ID: 4175d6e4-98fc-46f6-b549-c13e9f647eef | Oracle ID: ee45c80c-699c-4e37-97e6-1859c6601c99
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SCORIA_WURM,
    oracle_id = "ee45c80c-699c-4e37-97e6-1859c6601c99",
    scryfall_id = "4175d6e4-98fc-46f6-b549-c13e9f647eef",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Scoria Wurm",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WURM],
        power = Some(7),
        toughness = Some(7),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::FlipCoin {
            won: &[],
            lost: &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }],
        }],
    ),],
);
