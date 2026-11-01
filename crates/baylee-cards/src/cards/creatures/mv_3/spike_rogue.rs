//! Spike Rogue — {1}{G}{G} — Creature — Spike
//! Oracle: This creature enters with two +1/+1 counters on it.
//! Oracle: {2}, Remove a +1/+1 counter from this creature: Put a +1/+1 counter on target creature.
//! Oracle: {2}, Remove a +1/+1 counter from a creature you control: Put a +1/+1 counter on this creature.
//! Set: EXO #127 — Exodus | Scryfall ID: f0d9b671-344b-460d-8f65-d65129db91c3 | Oracle ID: 90ec956f-b6de-4214-9b5a-895280e542c1
// PARTIAL — Removing a +1/+1 counter from any creature you control as an activation cost is not supported; enters with two +1/+1 counters and can pay {2} and remove a +1/+1 counter from itself to put a +1/+1 counter on target creature

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: Removing a +1/+1 counter from any creature you control as an activation cost is not supported; enters with two +1/+1 counters and can pay {2} and remove a +1/+1 counter from itself to put a +1/+1 counter on target creature
card!(
    index = index::SPIKE_ROGUE,
    oracle_id = "90ec956f-b6de-4214-9b5a-895280e542c1",
    scryfall_id = "f0d9b671-344b-460d-8f65-d65129db91c3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Removing a +1/+1 counter from any creature you control as an activation cost is not supported; enters with two +1/+1 counters and can pay {2} and remove a +1/+1 counter from itself to put a +1/+1 counter on target creature",
    ),
    faces = &[face!(
        name = "Spike Rogue",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIKE],
        power = Some(0),
        toughness = Some(0),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(2),
        }],
    ),],
    abilities = &[activated!(
        cost!(
            "{2}",
            RemoveCounterSelf {
                kind: CounterKind::P1P1,
                n: 1,
            }
        ),
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
