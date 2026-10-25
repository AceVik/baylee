//! `cards/creatures/mv_4/lost_order_of_jarkeld.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const P2: PlayerId = PlayerId::new(2);

fn lost_order_of_jarkeld() -> CardIndex {
    card_index("75f047dc-4a51-494d-8765-fa7502d1fd06")
}

fn cast(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
    for color in ManaColor::ALL {
        engine.state.players[usize::from(seat.0)]
            .mana_pool
            .add(color, 10);
    }
    engine.refresh_offer();
    cast_with_floating(engine, seat, card);
}

fn choose_opponent(engine: &mut Engine<RegistryLookup>, opponent: PlayerId) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    engine
        .apply(P0, PlayerAction::ChoosePlayer(opponent))
        .expect("the chosen opponent is offered");
    pass_until(engine, stack_is_empty);
}

/// "As this creature enters, choose an opponent": the controller cannot
/// choose themselves. With no creatures belonging to that opponent, it is 1/1.
#[test]
fn lost_order_of_jarkeld_chooses_only_an_opponent_and_starts_at_one_without_their_creatures() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[plains(), sol_ring()])
        .hand(0, &[lost_order_of_jarkeld()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast(&mut engine, P0, lost_order_of_jarkeld());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    let Pending::ChoosePlayer { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options, vec![P1]);
    let order = on_battlefield(&engine, P0, lost_order_of_jarkeld()).unwrap();
    assert_eq!(
        engine.state().object(order).unwrap().chosen_opponent(),
        None
    );
    assert!(engine.apply(P0, PlayerAction::ChoosePlayer(P0)).is_err());
    assert!(
        matches!(engine.pending(), Pending::ChoosePlayer { player, options }
        if *player == P0 && options == &[P1])
    );
    assert_eq!(
        engine.state().object(order).unwrap().chosen_opponent(),
        None
    );
    choose_opponent(&mut engine, P1);

    assert_eq!(
        engine.state().object(order).unwrap().chosen_opponent(),
        Some(P1)
    );
    assert_eq!(pt(&engine, order), (1, 1), "only creatures of P1 count");
}

/// At a table of three, only the chosen player's creatures count. The size
/// follows creatures leaving and entering, rather than fixing the entry count.
#[test]
fn lost_order_of_jarkeld_tracks_only_the_chosen_players_current_creatures() {
    let mut engine = Duel::table(SEED, plains(), 3)
        .battlefield(0, &[llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .battlefield(
            2,
            &[
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
                forest(),
            ],
        )
        .hand(0, &[lost_order_of_jarkeld()])
        .hand(2, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast(&mut engine, P0, lost_order_of_jarkeld());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    let Pending::ChoosePlayer { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options, vec![P1, P2], "both opponents are choices");
    choose_opponent(&mut engine, P2);
    let order = on_battlefield(&engine, P0, lost_order_of_jarkeld()).unwrap();
    assert_eq!(
        pt(&engine, order),
        (4, 4),
        "three chosen creatures plus one"
    );

    let other = on_battlefield(&engine, P1, llanowar_elves()).unwrap();
    kill(&mut engine, other);
    assert_eq!(pt(&engine, order), (4, 4), "the other opponent is excluded");
    let mine = on_battlefield(&engine, P0, llanowar_elves()).unwrap();
    kill(&mut engine, mine);
    assert_eq!(pt(&engine, order), (4, 4), "the controller is excluded");
    let chosen = on_battlefield(&engine, P2, llanowar_elves()).unwrap();
    kill(&mut engine, chosen);
    assert_eq!(pt(&engine, order), (3, 3), "one chosen creature died");

    pass_until(&mut engine, |e| {
        e.state().turn.active == P2
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P2)
    });
    cast(&mut engine, P2, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, order),
        (4, 4),
        "a new chosen creature increases it"
    );
}

/// There is no chosen player in hand, on the initial stack, or after death.
/// Bouncing it erases the old choice, and the new permanent chooses again.
#[test]
fn lost_order_of_jarkeld_resets_its_choice_outside_the_battlefield_and_chooses_again() {
    let mut engine = Duel::table(SEED, plains(), 3)
        .battlefield(1, &[llanowar_elves()])
        .battlefield(2, &[llanowar_elves(), llanowar_elves(), llanowar_elves()])
        .hand(0, &[lost_order_of_jarkeld(), unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let order = in_hand(&engine, P0, lost_order_of_jarkeld()).unwrap();
    assert_eq!(
        pt(&engine, order),
        (1, 1),
        "no opponent has been chosen in hand"
    );
    cast(&mut engine, P0, lost_order_of_jarkeld());
    assert_eq!(on_stack(&engine, lost_order_of_jarkeld()), Some(order));
    assert_eq!(
        pt(&engine, order),
        (1, 1),
        "entry has not chosen a player yet"
    );
    choose_opponent(&mut engine, P1);
    assert_eq!(pt(&engine, order), (2, 2));

    cast(&mut engine, P0, unsummon());
    aim_at(&mut engine, P0, order);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(in_hand(&engine, P0, lost_order_of_jarkeld()), Some(order));
    assert_eq!(
        pt(&engine, order),
        (1, 1),
        "the former choice does not survive"
    );
    cast(&mut engine, P0, lost_order_of_jarkeld());
    assert_eq!(
        pt(&engine, order),
        (1, 1),
        "the second spell has no old choice"
    );
    choose_opponent(&mut engine, P2);
    assert_eq!(
        pt(&engine, order),
        (4, 4),
        "the new choice replaces the old one"
    );

    kill(&mut engine, order);
    assert_eq!(
        in_graveyard(&engine, P0, lost_order_of_jarkeld()),
        Some(order)
    );
    assert_eq!(
        pt(&engine, order),
        (1, 1),
        "the graveyard card has no choice"
    );
}
