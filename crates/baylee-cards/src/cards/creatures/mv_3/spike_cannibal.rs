//! Spike Cannibal — {1}{B}{B} — Creature — Spike
//! Oracle: This creature enters with a +1/+1 counter on it.
//! Oracle: When this creature enters, move all +1/+1 counters from all creatures onto it.
//! Set: EXO #75 — Exodus | Scryfall ID: 64003772-c62f-4728-a00c-48c78991c6ae | Oracle ID: c175ab0e-7085-41ec-83e4-38fc85c338fa
// PARTIAL — Moving only +1/+1 counters from all creatures onto this creature when it enters is not supported; enters with one +1/+1 counter

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: Moving only +1/+1 counters from all creatures onto this creature when it enters is not supported; enters with one +1/+1 counter
card!(
    index = index::SPIKE_CANNIBAL,
    oracle_id = "c175ab0e-7085-41ec-83e4-38fc85c338fa",
    scryfall_id = "64003772-c62f-4728-a00c-48c78991c6ae",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Moving only +1/+1 counters from all creatures onto this creature when it enters is not supported; enters with one +1/+1 counter",
    ),
    faces = &[face!(
        name = "Spike Cannibal",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIKE],
        power = Some(0),
        toughness = Some(0),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
    ),],
);
