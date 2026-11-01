//! Draw continuations, skipped steps, payment boundaries, and deterministic progress.
use super::*;

#[test]
fn replacement_draws_are_not_cards_drawn_by_the_original_instruction() {
    let mut e = game(&[DRAW, REVEAL]);
    let hand_before = e.state().zones.list(ZoneLocation::Hand(me())).len();
    start(&mut e, DRAW, 0);
    assert!(
        matches!(e.pending(), Pending::Priority { .. }),
        "both replacement draws leave no original card to discard"
    );
    settle(&mut e, me());
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before + 2
    );
}

#[test]
fn declining_denial_reopens_dredge_without_reapplying_the_same_replacement() {
    let mut e = game(&[DRAW, DENY, DREDGE]);
    let dredge = the(&e, ZoneLocation::Battlefield, DREDGE);
    let deny = the(&e, ZoneLocation::Battlefield, DENY);
    e.state
        .move_object(
            dredge,
            ZoneLocation::Graveyard(me()),
            ZonePosition::Top,
            Cause::Setup,
        )
        .unwrap();
    let before = e.state().per_turn.draws[0];
    start(&mut e, DRAW, 1);
    choose(&mut e, vec![deny]);
    e.apply(them(), PlayerAction::YesNo(false)).unwrap();
    assert!(
        matches!(e.pending(), Pending::ChooseCards { options, min: 0, max: 1, .. } if options == &vec![dredge])
    );
    choose(&mut e, vec![dredge]);
    settle(&mut e, me());
    assert_eq!(e.state().per_turn.draws[0], before);
    assert_eq!(e.state().object(dredge).unwrap().zone, Zone::Hand);
}

#[test]
fn denial_uses_apnap_and_stops_after_the_first_payment() {
    let third = PlayerId::new(2);
    let mut e = bench(
        7980,
        cards(),
        [Seat::with(&[DRAW, DENY]), Seat::default(), Seat::default()],
    );
    // Three-player games do not skip the starting player's first draw step.
    // Decline its two payment offers before arranging the activated-draw probe.
    walk_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    for player in [them(), third] {
        e.apply(player, PlayerAction::YesNo(false)).unwrap();
    }
    to_main(&mut e, me());
    let card = top(&e);
    let source = the(&e, ZoneLocation::Battlefield, DRAW);
    activate(&mut e, me(), source, 1);
    walk_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    assert!(matches!(e.pending(), Pending::YesNo { player, .. } if *player == them()));
    e.apply(them(), PlayerAction::YesNo(false)).unwrap();
    assert!(matches!(e.pending(), Pending::YesNo { player, .. } if *player == third));
    let life = e.state().players[2].life;
    e.apply(third, PlayerAction::YesNo(true)).unwrap();
    settle(&mut e, me());
    assert_eq!(e.state().players[2].life, life - 2);
    assert_eq!(e.state().object(card).unwrap().zone, Zone::Graveyard);
}

#[test]
fn delayed_hidden_return_ignores_the_opponents_end_step() {
    let mut e = game(&[EXILER]);
    to_main(&mut e, them());
    e.apply(them(), PlayerAction::PassPriority).unwrap();
    let card = top(&e);
    start(&mut e, EXILER, 0);
    settle(&mut e, them());
    walk_until(&mut e, |e| {
        e.state().turn.step == Step::End && e.state().zones.stack_is_empty()
    });
    assert_eq!(e.state().object(card).unwrap().zone, Zone::Exile);
    to_main(&mut e, me());
    walk_until(&mut e, |e| {
        e.state().turn.step == Step::End && e.state().zones.stack_is_empty()
    });
    assert_eq!(e.state().object(card).unwrap().zone, Zone::Hand);
}

#[test]
fn skip_draw_step_suppresses_the_draw_and_step_trigger_but_not_other_draws() {
    let mut e = game(&[DRAW, SKIPPER]);
    let life = e.state().players[0].life;
    let before = e.state().zones.list(ZoneLocation::Hand(me())).len();
    to_main(&mut e, them());
    to_main(&mut e, me());
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(me())).len(), before);
    assert_eq!(e.state().players[0].life, life);
    start(&mut e, DRAW, 1);
    settle(&mut e, me());
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(me())).len(),
        before + 1
    );
    start(&mut e, SKIPPER, 2);
    settle(&mut e, me());
    to_main(&mut e, them());
    to_main(&mut e, me());
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(me())).len(),
        before + 2
    );
}

#[test]
fn draw_and_delayed_progress_distinguish_snapshots_and_loop_situations() {
    let mut e = game(&[DRAW, DENY, EXILER]);
    start(&mut e, DRAW, 1);
    let snapshot = e.snapshot_hash();
    let loop_before = e.state().loop_signature();
    e.state.draw_front.denial.as_mut().unwrap().2.clear();
    assert_ne!(e.snapshot_hash(), snapshot);
    assert_ne!(e.state().loop_signature(), loop_before);
    e.apply(them(), PlayerAction::YesNo(false)).unwrap();
    settle(&mut e, me());
    start(&mut e, EXILER, 0);
    settle(&mut e, me());
    let snapshot = e.snapshot_hash();
    let loop_before = e.state().loop_signature();
    e.state.delayed.clear();
    assert_ne!(e.snapshot_hash(), snapshot);
    assert_ne!(e.state().loop_signature(), loop_before);
}

#[test]
fn loot_hashes_preserve_choices_and_normalize_allocated_draw_identities() {
    let mut first = bench(
        7980,
        cards(),
        [Seat::with(&[DRAW]).holding(&[PLAIN]), Seat::default()],
    );
    let mut second = bench(
        7981,
        cards(),
        [Seat::with(&[DRAW]).holding(&[PLAIN]), Seat::default()],
    );
    for e in [&mut first, &mut second] {
        to_main(e, me());
        start(e, DRAW, 0);
    }
    let first_progress =
        crate::resolve::loot::loop_progress(first.state(), first.resolution.as_ref().unwrap());
    let second_progress =
        crate::resolve::loot::loop_progress(second.state(), second.resolution.as_ref().unwrap());
    assert_eq!(
        first_progress, second_progress,
        "the same hand positions are the same choice, regardless of card IDs"
    );
    assert_eq!(
        first.state().loop_signature(),
        second.state().loop_signature()
    );
    let before = first.snapshot_hash();
    let card = match first.pending() {
        Pending::ChooseCards { options, .. } => options[0],
        _ => unreachable!(),
    };
    let Some(crate::resolve::AwaitingOp::Loot(loot)) =
        &mut first.resolution.as_mut().unwrap().awaiting
    else {
        panic!("loot selection")
    };
    loot.selected.push(card);
    assert_ne!(first.snapshot_hash(), before);
    assert_ne!(
        crate::resolve::loot::loop_progress(first.state(), first.resolution.as_ref().unwrap()),
        first_progress
    );
    let before = second.state().snapshot_hash();
    second.state.per_turn.instruction_drawn.clear();
    assert_ne!(second.state().snapshot_hash(), before);
}

#[test]
fn unpaid_life_cannot_be_offered_as_a_draw_denial() {
    for locked in [false, true] {
        let opponent = if locked {
            Seat::with(&[LOCK])
        } else {
            Seat::default().at(1)
        };
        let mut e = bench(7980, cards(), [Seat::with(&[DRAW, DENY]), opponent]);
        to_main(&mut e, me());
        let card = top(&e);
        start(&mut e, DRAW, 1);
        assert!(matches!(e.pending(), Pending::Priority { .. }));
        settle(&mut e, me());
        assert_eq!(e.state().object(card).unwrap().zone, Zone::Hand);
    }
}

#[test]
fn unknown_discard_still_triggers_but_its_library_successor_is_not_exiled() {
    let mut e = game(&[DRAW, EXILER, LENG]);
    let card = the(&e, ZoneLocation::Hand(me()), PLAIN);
    let source = the(&e, ZoneLocation::Battlefield, EXILER);
    start(&mut e, DRAW, 2);
    assert!(matches!(e.pending(), Pending::Arrange { .. }));
    e.apply(
        me(),
        PlayerAction::Arrange {
            piles: vec![vec![], vec![card]],
        },
    )
    .unwrap();
    assert!(e.journal().entries().iter().any(|entry| matches!(entry.event, crate::event::GameEvent::AbilityTriggered { source: origin, .. } if origin == source)), "a discard without known characteristics still triggers an unfiltered observer");
    settle(&mut e, me());
    assert_eq!(e.state().object(card).unwrap().zone, Zone::Library);
}
