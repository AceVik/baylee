//! `cards/creatures/artifacts/mv_1/esper_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::PriorityHold;

fn esper_sentinel_card() -> CardIndex {
    card_index("5def9f38-0a0b-4e8d-9f9d-29dcb46520b4")
}

/// "Creatures you control get +1/+1."
fn anthem() -> CardIndex {
    card_index("e3886fe8-9b76-4613-8891-4ec74657c087")
}

fn hand_of(engine: &Engine<RegistryLookup>, seat: u8) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Hand(PlayerId::new(seat)))
        .len()
}

fn is_tax_question(e: &Engine<RegistryLookup>) -> bool {
    matches!(
        e.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::PayTax { .. },
            ..
        }
    )
}

/// A table of four on seat 2's main phase: seat 0 has the Sentinel (and
/// `extra` beside it), seat 2 has `forests` Forests and two Sol Rings and an
/// Elves in hand. Nothing has been cast.
fn seat_two_in_its_main_with(extra: &[CardIndex], forests: usize) -> Engine<RegistryLookup> {
    let mut board = vec![esper_sentinel_card()];
    board.extend_from_slice(extra);
    let mut engine = Duel::table(SEED, island(), 4)
        .battlefield(0, &board)
        .battlefield(2, &vec![forest(); forests])
        .hand(2, &[llanowar_elves(), sol_ring(), sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, PlayerId::new(2));
    engine
}

/// The whole sentence on one board at a table of four.
///
/// - A creature spell is not a noncreature spell: seat 2's Elves cast first
///   asks nothing (`pass_until` panics on a tax question it was not told to
///   expect).
/// - The first noncreature spell (Sol Ring) asks seat 2 and only seat 2, for
///   {X} where X is the Sentinel's power: an anthem on the table makes it a
///   2/2 and the tax {2}, not the {1} of a bare 1/1.
/// - Declined, its controller draws. This is not a "may".
/// - A second noncreature spell the same turn (the second Sol Ring) asks
///   nothing and draws nothing: "first noncreature spell each turn".
#[test]
fn esper_sentinel_taxes_the_casters_first_noncreature_spell_at_its_power() {
    let (p0, p2) = (PlayerId::new(0), PlayerId::new(2));
    let mut engine = seat_two_in_its_main_with(&[anthem()], 5);
    let sentinel = on_battlefield(&engine, p0, esper_sentinel_card()).expect("the Sentinel");
    assert_eq!(pt(&engine, sentinel), (2, 2), "the anthem makes X = 2");
    let hand0 = hand_of(&engine, 0);

    // Five Forests float {G}x5 and each spell takes what it costs.
    cast_from_hand(&mut engine, p2, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_of(&engine, 0),
        hand0,
        "a creature spell does not trigger the Sentinel"
    );

    cast_with_floating(&mut engine, p2, sol_ring());
    pass_until(&mut engine, is_tax_question);
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending()
    else {
        unreachable!("the predicate matched a tax question");
    };
    assert_eq!(*player, p2, "the caster is asked, not seat 1");
    assert_eq!(*mana, 2, "X is the Sentinel's power, which is 2");
    assert_eq!(
        engine.awaited().iter().collect::<Vec<_>>(),
        vec![p2],
        "no other seat has a question"
    );
    for other in [0, 1, 3] {
        assert!(
            engine.pending_for(PlayerId::new(other)).is_none(),
            "seat {other} is asked nothing"
        );
    }

    engine.apply(p2, PlayerAction::YesNo(false)).unwrap();
    assert!(
        !matches!(engine.pending(), Pending::YesNo { .. }),
        "unpaid is a plain draw for the controller, with no \"may\" to answer: {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_of(&engine, 0),
        hand0 + 1,
        "the tax went unpaid, so seat 0 drew"
    );

    // The second noncreature spell of the turn. Nothing may ask: `pass_until`
    // would panic on the question.
    cast_with_floating(&mut engine, p2, sol_ring());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_of(&engine, 0),
        hand0 + 1,
        "a second noncreature spell the same turn draws nothing"
    );
}

/// Paid, with the Sentinel a bare 1/1: the question is for {1}, the payment
/// takes exactly that, and nobody draws.
#[test]
fn esper_sentinel_paid_draws_nothing_and_a_bare_one_one_asks_for_one() {
    let (p0, p2) = (PlayerId::new(0), PlayerId::new(2));
    let mut engine = seat_two_in_its_main_with(&[], 3);
    let sentinel = on_battlefield(&engine, p0, esper_sentinel_card()).expect("the Sentinel");
    assert_eq!(pt(&engine, sentinel), (1, 1), "X = 1");
    let hand0 = hand_of(&engine, 0);

    cast_from_hand(&mut engine, p2, sol_ring());
    pass_until(&mut engine, is_tax_question);
    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana: 1 },
                ..
            } if *player == p2
        ),
        "seat 2 is asked for {{1}}: {:?}",
        engine.pending()
    );
    // Three Forests float three mana; Sol Ring took one.
    assert_eq!(engine.state().players[2].mana_pool.total(), 2);
    engine.apply(p2, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[2].mana_pool.total(),
        1,
        "the tax took {{1}}"
    );
    assert_eq!(hand_of(&engine, 0), hand0, "paid, so nobody draws");
}

/// The report, end to end at a table of four. Seat 2 casts its first
/// noncreature spell with one Forest tapped and one left standing, so the
/// pool is empty when the Sentinel asks for {1}, and it has told the game to
/// resolve the stack for it (`UntilStackEmpty { depth: 2 }`, set while the Sol
/// Ring and the trigger are both on it).
///
/// Saying yes opens the payment window with seat 2 holding priority in it.
/// The old engine let the hold pass that priority, which closed the window
/// with nothing paid, and seat 0 drew. Now the window stays open for the seat
/// to tap the Forest and pass, the {1} comes out of that mana, and nobody
/// draws. The hold is checked alive at the question: one that had already
/// expired would have passed nothing, on old code too.
#[test]
fn esper_sentinel_tax_said_yes_to_under_a_stack_hold_is_paid_from_the_window() {
    let (p0, p2) = (PlayerId::new(0), PlayerId::new(2));
    let mut engine = seat_two_in_its_main_with(&[], 2);
    let hand0 = hand_of(&engine, 0);
    let spent = all_on_battlefield(&engine, p2, forest())[0];
    tap_mana_where(&mut engine, p2, |id| id == spent);

    cast_with_floating(&mut engine, p2, sol_ring());
    assert_eq!(
        engine.state().players[2].mana_pool.total(),
        0,
        "the Forest paid for the Sol Ring, so the tax meets an empty pool"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        2,
        "the Sol Ring and the Sentinel's trigger"
    );
    let hold = PriorityHold::UntilStackEmpty { depth: 2 };
    engine
        .apply(p2, PlayerAction::SetPriorityHold(hold))
        .expect("setting a hold is always legal");

    pass_until(&mut engine, is_tax_question);
    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana: 1 },
                ..
            } if *player == p2
        ),
        "seat 2 is asked for {{1}}: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.automation(p2).hold,
        hold,
        "the hold is alive at the question: the Sol Ring is still on the stack"
    );

    engine.apply(p2, PlayerAction::YesNo(true)).unwrap();
    assert!(
        matches!(engine.payment_window(), Some((seat, _)) if seat == p2),
        "the yes opened a window and the hold left it open: {:?}",
        engine.pending()
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p2),
        "seat 2 holds priority in its window: {:?}",
        engine.pending()
    );
    assert_eq!(hand_of(&engine, 0), hand0, "nothing is unpaid yet");

    assert_eq!(tap_all_mana(&mut engine, p2), 1, "the Forest left standing");
    engine.apply(p2, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[2].mana_pool.total(),
        0,
        "the tax took the {{1}} the Forest made"
    );
    assert_eq!(
        hand_of(&engine, 0),
        hand0,
        "paid, so seat 0 (the Sentinel's controller) drew nothing"
    );
    assert!(
        on_battlefield(&engine, p2, sol_ring()).is_some(),
        "the Sol Ring resolved"
    );
    assert!(
        on_battlefield(&engine, p0, esper_sentinel_card()).is_some(),
        "the Sentinel is still there"
    );
}
