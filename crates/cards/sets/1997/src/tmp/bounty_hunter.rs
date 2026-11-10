//! Bounty Hunter — {2}{B}{B} — Creature — Human Archer Minion
//! Oracle: {T}: Put a bounty counter on target nonblack creature.
//! Oracle: {T}: Destroy target creature with a bounty counter on it.
//! Set: TMP #110 — Tempest | Scryfall ID: 98319fd3-0aad-4fc3-bb83-3c027d0ed652 | Oracle ID: 2fd1cfc5-fbcd-4d31-ba8b-759d8c41b435
// IMPLEMENTED — Put a bounty counter on target nonblack creature, or destroy target creature with a bounty counter on it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const NONBLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
]);

const CREATURE_WITH_BOUNTY: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasCounter(counters::BOUNTY)]);

card!(
    index = index::BOUNTY_HUNTER,
    oracle_id = "2fd1cfc5-fbcd-4d31-ba8b-759d8c41b435",
    scryfall_id = "98319fd3-0aad-4fc3-bb83-3c027d0ed652",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Bounty Hunter",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::ARCHER,
            subtypes::creature::MINION
        ],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        activated!(
            Cost::TAP,
            &[Effect::AddCounter {
                kind: counters::BOUNTY,
                amount: Amount::Fixed(1),
            }],
            target = Some(TargetSpec::Object(&NONBLACK_CREATURE)),
        ),
        activated!(
            Cost::TAP,
            &[Effect::destroy(TargetSpec::Object(&CREATURE_WITH_BOUNTY))],
            target = Some(TargetSpec::Object(&CREATURE_WITH_BOUNTY)),
        ),
    ],
);
