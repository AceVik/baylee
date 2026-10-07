//! "That player" of a trigger whose event names a player: the one who drew,
//! the one who cast (`PlayerRel::EventPlayer`).
//!
//! The event is what the ability triggered on (CR 603.2), and "that player"
//! or "they" is the one player that event was about — never "each
//! opponent". Heads-up the two are one seat, which is how Sheoldred, the
//! Apocalypse drained every opponent and Rhystic Study asked the first
//! opponent in seat order to pay for a spell somebody else cast, both with
//! every two-player test green (owner, 07.10.2026). So these are played at a
//! table of four, with the event's player chosen so that it is *not* the
//! lowest-numbered opponent.

use super::testkit::{
    Duel, card_index, cast_from_hand, keep_mulligans, pass_until, reach_their_main_phase,
    stack_is_empty,
};
use super::*;
use crate::choice::YesNoPrompt;
use crate::zone::ZoneLocation;
use baylee_core::ids::CardIndex;

const SEED: u64 = 0x7a47_91a7;

fn seat(n: u8) -> PlayerId {
    PlayerId::new(n)
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// "Whenever you draw a card, you gain 2 life. Whenever an opponent draws a
/// card, they lose 2 life."
fn sheoldred_the_apocalypse() -> CardIndex {
    card_index("34f34409-326d-4994-a0ea-1a69aa278f03")
}

/// "Whenever an opponent casts a spell, you may draw a card unless that
/// player pays {1}."
fn rhystic_study() -> CardIndex {
    card_index("53236dd7-845a-444c-96d5-f41ed7325d8f")
}

fn lives(engine: &Engine<testkit::RegistryLookup>) -> Vec<i32> {
    engine.state().players.iter().map(|p| p.life).collect()
}

/// Each opponent's draw step costs that opponent 2 life, and only that one:
/// seat 1 draws and loses 2 while seats 2 and 3 keep theirs, then seat 2
/// draws and loses 2 while seat 1 loses nothing more.
#[test]
fn an_opponent_who_draws_is_the_only_one_who_loses_life() {
    let mut engine = Duel::table(SEED, island(), 4)
        .battlefield(0, &[sheoldred_the_apocalypse()])
        .start();
    keep_mulligans(&mut engine);
    let start = lives(&engine);

    reach_their_main_phase(&mut engine, seat(1));
    pass_until(&mut engine, stack_is_empty);
    let after_one = lives(&engine);
    assert_eq!(
        after_one[1..],
        [start[1] - 2, start[2], start[3]],
        "seat 1 drew: seat 1 loses 2 and nobody else does"
    );

    reach_their_main_phase(&mut engine, seat(2));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        lives(&engine)[1..],
        [start[1] - 2, start[2] - 2, start[3]],
        "seat 2 drew: seat 2 loses 2, seat 1 nothing more, seat 3 nothing"
    );
}

/// Seat 2 casts a spell into seat 0's Rhystic Study: seat 2 is the one asked
/// to pay {1}, and while that question is open no other seat is awaited.
/// Seat 1, the first opponent in seat order, is asked nothing.
#[test]
fn only_the_player_who_cast_is_asked_to_pay() {
    let mut engine = Duel::table(SEED, island(), 4)
        .battlefield(0, &[rhystic_study()])
        .battlefield(2, &[forest()])
        .hand(2, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, seat(2));
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(seat(0))).len();
    cast_from_hand(&mut engine, seat(2), llanowar_elves());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });

    let Pending::YesNo { player, .. } = engine.pending() else {
        unreachable!("the predicate above matched a YesNo");
    };
    assert_eq!(*player, seat(2), "the caster is asked to pay, not seat 1");
    assert_eq!(
        engine.awaited().iter().collect::<Vec<_>>(),
        vec![seat(2)],
        "no other seat has a question"
    );
    for other in [seat(0), seat(1), seat(3)] {
        assert!(
            engine.pending_for(other).is_none(),
            "{other:?} is asked nothing"
        );
    }

    // The caster declines: Rhystic Study's controller may draw. The pass
    // answers "you may" with yes.
    engine.apply(seat(2), PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(seat(0))).len(),
        hand_before + 1,
        "the tax went unpaid, so seat 0 drew"
    );
}
