//! Aerathi Berserker — {2}{R}{R}{R} — Creature — Human Berserker
//! Oracle: Rampage 3 (Whenever this creature becomes blocked, it gets +3/+3 until end of turn for each creature blocking it beyond the first.)
//! Set: LEG #131 — Legends | Scryfall ID: 06673800-22a7-4ee3-92fa-7c7cd4865d30 | Oracle ID: 5cb495a2-c683-4066-b6ee-d0b7d8843cb9

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AERATHI_BERSERKER,
    oracle_id = "5cb495a2-c683-4066-b6ee-d0b7d8843cb9",
    scryfall_id = "06673800-22a7-4ee3-92fa-7c7cd4865d30",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::Rampage,
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Times {
                base: &Amount::BlockersBeyondFirst,
                factor: 3
            },
            toughness: Amount::Times {
                base: &Amount::BlockersBeyondFirst,
                factor: 3
            },
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }]
    )],
    faces = &[face!(
        name = "Aerathi Berserker",
        mana_cost = mana!("{2}{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::BERSERKER],
        power = Some(2),
        toughness = Some(4),
    ),],
);
