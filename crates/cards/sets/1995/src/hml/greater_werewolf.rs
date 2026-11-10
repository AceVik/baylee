//! Greater Werewolf — {4}{B} — Creature — Werewolf
//! Oracle: At end of combat, put a -0/-2 counter on each creature blocking or blocked by this creature.
//! Set: 5ED #166 — Fifth Edition | Scryfall ID: 34b77373-6066-436b-aa60-4cb0f8077599 | Oracle ID: 02847b0d-03a1-4e30-902e-60bdf6952c58
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const BLOCKING_OR_BLOCKED_BY_SOURCE: Filter =
    Filter::Or(&[Filter::BlockingSource, Filter::BlockedBySource]);

card!(
    index = index::GREATER_WEREWOLF,
    oracle_id = "02847b0d-03a1-4e30-902e-60bdf6952c58",
    scryfall_id = "34b77373-6066-436b-aa60-4cb0f8077599",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Greater Werewolf",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WEREWOLF],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::CombatEnd,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::AddCounterFilter {
            filter: &BLOCKING_OR_BLOCKED_BY_SOURCE,
            kind: CounterKind::Minus {
                power: 0,
                toughness: 2,
            },
            amount: Amount::Fixed(1),
        }],
    )],
);
