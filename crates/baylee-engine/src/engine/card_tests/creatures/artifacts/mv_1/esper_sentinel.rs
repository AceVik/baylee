//! `cards/creatures/artifacts/mv_1/esper_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

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
