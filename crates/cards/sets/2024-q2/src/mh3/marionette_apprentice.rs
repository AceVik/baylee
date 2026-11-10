//! Marionette Apprentice — {1}{B} — Creature — Human Artificer
//! Oracle: Fabricate 1 (When this creature enters, put a +1/+1 counter on it or create a 1/1 colorless Servo artifact creature token.)
//! Oracle: Whenever another creature or artifact you control is put into a graveyard from the battlefield, each opponent loses 1 life.
//! Set: MH3 #100 — Modern Horizons 3 | Scryfall ID: d16f8670-f038-400a-83e7-a53a7f8c47ac | Oracle ID: 726d9d2c-736a-4852-9938-a0f50d8fd89f

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ANOTHER_ARTIFACT_OR_CREATURE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::ARTIFACT_OR_CREATURE,
    Filter::ControlledByYou,
    Filter::Another,
]);

card!(
    index = index::MARIONETTE_APPRENTICE,
    oracle_id = "726d9d2c-736a-4852-9938-a0f50d8fd89f",
    scryfall_id = "d16f8670-f038-400a-83e7-a53a7f8c47ac",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Marionette Apprentice",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ARTIFICER],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[
        triggered!(
            Trigger::Dies(&ANOTHER_ARTIFACT_OR_CREATURE_YOU_CONTROL),
            &[Effect::LoseLife {
                amount: Amount::Fixed(1),
                target: PlayerRel::EachOpponent,
            }]
        ),
        triggered!(
            Trigger::ETB,
            &[Effect::Fabricate {
                count: 1,
                token: &crate::tokens::SERVO_1_1
            }]
        ),
    ],
);
