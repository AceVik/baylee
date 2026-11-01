//! Mindwarper — {2}{B}{B} — Creature — Spirit
//! Oracle: This creature enters with three +1/+1 counters on it.
//! Oracle: {2}{B}, Remove a +1/+1 counter from this creature: Target player discards a card. Activate only as a sorcery.
//! Set: STH #64 — Stronghold | Scryfall ID: ebe796e1-97f9-469a-a6b6-a09161058e12 | Oracle ID: efb42ccc-6fed-47f0-84b4-e2a9a42e4dfa
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MINDWARPER,
    oracle_id = "efb42ccc-6fed-47f0-84b4-e2a9a42e4dfa",
    scryfall_id = "ebe796e1-97f9-469a-a6b6-a09161058e12",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mindwarper",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(0),
        toughness = Some(0),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(3),
        }],
    ),],
    abilities = &[activated!(
        cost!(
            "{2}{B}",
            RemoveCounterSelf {
                kind: CounterKind::P1P1,
                n: 1,
            }
        ),
        &[Effect::DiscardForPlayers {
            who: PlayerRel::Chosen,
            count: 1,
        }],
        timing = ActivationTiming::SorcerySpeed,
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
