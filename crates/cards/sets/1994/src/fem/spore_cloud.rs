//! Spore Cloud — {1}{G}{G} — Instant
//! Oracle: Tap all blocking creatures. Prevent all combat damage that would be dealt this turn. Each attacking creature and each blocking creature doesn't untap during its controller's next untap step.
//! Set: ME2 #176 — Masters Edition II | Scryfall ID: 7a2cd32d-dea1-4d58-b8d6-5d7176c967f2 | Oracle ID: 36d55084-4696-4d16-ab3a-b00f1456603f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const BLOCKING_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Blocking]);
const ATTACKING_OR_BLOCKING_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Or(&[Filter::Attacking, Filter::Blocking]),
]);

card!(
    index = index::SPORE_CLOUD,
    oracle_id = "36d55084-4696-4d16-ab3a-b00f1456603f",
    scryfall_id = "7a2cd32d-dea1-4d58-b8d6-5d7176c967f2",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Spore Cloud",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[
        Effect::TapAll {
            filter: &BLOCKING_CREATURE,
        },
        Effect::PreventAllCombatDamageThisTurn,
        Effect::SuppressUntap {
            target: TargetSpec::Object(&ATTACKING_OR_BLOCKING_CREATURE),
            steps: 1,
        },
    ])],
);
