//! Decision clocks, mulligans, and the house taking an empty chair.

use super::*;

/// A loader that drops and dials again before the table is open holds
/// none of the log its first socket was sent, so its second is told the
/// log from the first line (#256, #262), as a socket attaching to an open
/// table is.
///
/// Nothing the runner does before the curtain writes a line today: no
/// clock runs, no answer is applied, and the opening deal is not logged.
/// So the line is put there by hand, the one a pre-curtain table would
/// hold first if a clock ever ran there: the house standing in.
#[test]
fn a_seat_that_dials_again_before_the_curtain_is_told_its_log_from_the_start() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(600));
    assert!(
        logs(&attach(&mut runner, 0), 0).is_empty(),
        "the runner logs nothing before the curtain"
    );
    detach(&mut runner, 0);
    let session = runner.session.as_mut().expect("set up");
    assert!(
        session.stand_in(PlayerId::new(1)),
        "seat 1 is a human chair"
    );
    assert_eq!(
        logs(&attach(&mut runner, 0), 0),
        [(0, 1)],
        "the socket is told the line"
    );
    detach(&mut runner, 0);
    assert!(runner.curtain_pending(), "still loading");
    assert_eq!(
        logs(&attach(&mut runner, 0), 0),
        [(0, 1)],
        "the second socket was not told the log from its first line"
    );
}

/// During the opening mulligans each human seat is on its own clock, and
/// one seat's answer leaves the other's exactly as it was: the same
/// `Clock`, which is what keeps the attach loop from arming it again
/// ([`Armed::sync`]). The old deadline of the seat that answered answers
/// nothing, and the other seat's still answers for it.
#[test]
fn each_seat_deciding_its_mulligan_is_on_its_own_clock() {
    let (zero, one) = (PlayerId::new(0), PlayerId::new(1));
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(30));
    sit(&mut runner, 0);
    sit(&mut runner, 1);
    let before = runner.clocks();
    assert_eq!(
        before.iter().map(|c| (c.seat, c.what)).collect::<Vec<_>>(),
        [(zero, Deadline::Decide), (one, Deadline::Decide)]
    );
    act(&mut runner, 0, &PlayerAction::MulliganTake);
    let after = runner.clocks();
    assert_eq!(after[1], before[1], "seat 0's take re-armed seat 1's clock");
    assert_ne!(after[0], before[0], "seat 0 is asked anew after its take");
    assert!(
        runner.timeout(before[0]).is_empty(),
        "a stale deadline answered"
    );
    assert!(!runner.timeout(before[1]).is_empty());
    let session = runner.session().expect("a game");
    assert!(!session.awaited().contains(one), "seat 1 did not keep");
    assert_eq!(runner.clocks(), vec![after[0]]);
}

/// A deadline whose clock still runs unchanged keeps its moment; one that
/// changed is armed anew; one that stopped is gone.
#[test]
fn a_running_clock_keeps_the_deadline_it_was_armed_with() {
    let clock = |seat: u8, seq: u64| Clock {
        seat: PlayerId::new(seat),
        what: Deadline::Decide,
        seq,
        secs: 30,
    };
    let mut armed = Armed::default();
    armed.sync(&[clock(0, 0), clock(1, 0)], |c| 100 + u64::from(c.secs));
    // Ten seconds on, seat 0 is asked something new and seat 1 is not.
    armed.sync(&[clock(0, 1), clock(1, 0)], |c| 110 + u64::from(c.secs));
    assert_eq!(armed.next(), Some((clock(1, 0), 130)));
    assert_eq!(
        armed.remaining(|at| u32::try_from(at).unwrap_or(u32::MAX)),
        vec![(clock(1, 0), 130), (clock(0, 1), 140)]
    );
    armed.fired(clock(1, 0));
    assert_eq!(armed.next(), Some((clock(0, 1), 140)));
    armed.sync(&[], |_| 0);
    assert_eq!(armed.next(), None);
}

/// The clock belongs to the seat being asked, and nobody else may wind it.
///
/// A priority hold is one of the settings the engine takes from a seat
/// that is not on the clock — an ability's yield or standing answer is
/// another. Both produce frames without moving the game, so a clock
/// anchored to the frame counter restarts on every press of `F6` at the
/// other end of the table: unlimited thinking time for whoever spams it,
/// which is a cheat rather than a bug.
#[test]
fn the_other_seats_hold_does_not_wind_the_clock() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(30));
    sit(&mut runner, 0);
    sit(&mut runner, 1);
    // Past the mulligans, which ask both seats at once: this is about
    // the seat that is not being asked.
    keep_both(&mut runner);
    let before = on_clock(&runner).expect("a seat is on the clock");
    let frames_before = runner.session().expect("a game").seq();
    let idle = u32::from(before.seat.get() == 0);
    for _ in 0..3 {
        act(
            &mut runner,
            idle,
            &PlayerAction::SetPriorityHold(baylee_engine::choice::PriorityHold::UntilEndOfTurn {
                turn: 1,
            }),
        );
    }
    // Both halves, or the test proves nothing: an unchanged clock is also
    // what a *refused* hold would look like, and a hold that never reached
    // the engine could not be taken back either.
    assert_eq!(
        runner.session().expect("a game").seq(),
        frames_before + 3,
        "the seat that was not being asked could not state a hold at all"
    );
    assert_eq!(
        on_clock(&runner),
        Some(before),
        "the seat not being asked wound the other seat's clock"
    );
}

/// The clock is the one thing the rules kernel must not own, and it must
/// not run against a player who is not there to see it.
///
/// With another player at the table they are on the *other* clock instead:
/// a seat with no socket cannot lose on time to a question it never saw,
/// but the player still here must not be left waiting on it forever.
#[test]
fn nobody_is_on_a_clock_they_cannot_see() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans_window(60, 30));
    sit_both(&mut runner);
    let clock = clock_of(&runner, 0).expect("the seat being asked is here");
    assert_eq!(clock.what, Deadline::Decide);
    assert_eq!(clock.secs, 60);

    let told = detach(&mut runner, 0);
    let clock = clock_of(&runner, 0).expect("the table is still waiting on it");
    assert_eq!(
        clock.what,
        Deadline::StandIn,
        "the player walked away; it is the chair that is on a clock now"
    );
    assert_eq!(clock.secs, 30, "and it is the table's reconnect window");
    assert!(
        log_text(&told, 1).contains(r#"{"ConnectionLost":{"player":0,"wait_secs":30}}"#),
        "the player still here was not told: {}",
        log_text(&told, 1)
    );
}

/// A table with no decision limit puts nobody on a *decision* clock — and
/// the two limits are independent, because a table that gives its players
/// all the time in the world still must not sit forever on a closed
/// laptop while another player waits.
#[test]
fn no_limit_means_no_clock() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans_window(0, 30));
    sit_both(&mut runner);
    assert!(runner.clocks().is_empty());

    detach(&mut runner, 0);
    assert_eq!(
        clock_of(&runner, 0).map(|c| (c.what, c.secs)),
        Some((Deadline::StandIn, 30)),
        "no decision limit is not no reconnect window"
    );
}

/// A table that says to wait forever is waited on forever.
#[test]
fn a_table_that_never_gives_up_a_chair_never_takes_one() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel_window(60, 0));
    attach(&mut runner, 0);
    detach(&mut runner, 0);
    assert_eq!(on_clock(&runner), None);
}

/// With nobody else at the table a lost connection pauses the game (owner,
/// 10.10.2026): no decision clock, no reconnect window, no house move —
/// only the table's hold, a day long. The log says it is paused.
#[test]
fn a_player_alone_with_the_house_pauses_the_game() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel_window(60, 30));
    sit(&mut runner, 0);
    let before = runner.session().expect("built").decision_seq();
    let frames = runner.session().expect("built").seq();

    let told = detach(&mut runner, 0);
    assert!(
        told.is_empty(),
        "nobody is here to be told, and nothing moved"
    );
    let clock = on_clock(&runner).expect("the hold");
    assert_eq!((clock.what, clock.secs), (Deadline::Hold, HOLD_SECS));
    let session = runner.session().expect("built");
    assert_eq!(
        session.decision_seq(),
        before,
        "the house moved while paused"
    );
    assert_eq!(session.seq(), frames);
    assert!(
        !session
            .seat_kind(PlayerId::new(0))
            .is_some_and(SeatKind::is_away),
        "the house sat down at a paused table"
    );

    // Back: the seat is asked again on its decision clock, and its log says
    // what happened while it was gone.
    let back = sit(&mut runner, 0);
    assert_eq!(on_clock(&runner).map(|c| c.what), Some(Deadline::Decide));
    let said = log_text(&back, 0);
    assert!(
        said.contains(r#"{"ConnectionLost":{"player":0,"wait_secs":null}}"#)
            && said.contains(r#"{"Returned":{"player":0}}"#),
        "{said}"
    );
    assert_eq!(runner.session().expect("built").decision_seq(), before);
}

/// The hold is not forever: a day with nobody at the table, and the house
/// concedes for the absent player.
#[test]
fn a_paused_game_held_a_whole_day_ends() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel_window(60, 30));
    sit(&mut runner, 0);
    detach(&mut runner, 0);
    let hold = on_clock(&runner).expect("the hold");
    let out = runner.timeout(hold);
    assert!(runner.finished(), "the game was not ended");
    assert!(
        out.iter().any(
            |e| matches!(&e.msg, Some(v1::envelope::Msg::GameEnded(ended)) if ended.winners == [1])
        ),
        "the house's seat wins"
    );
}

/// The bug the reconnect window exists for: seat 0 closes its laptop
/// while it owes an answer, and the player still at the table waits.
///
/// A stand-in rather than one answer on the seat's behalf, because a
/// player who is not there for this question is not there for the next
/// one either — answering once would put the table straight back where it
/// was, one decision later.
#[test]
fn a_chair_nobody_is_sitting_in_goes_to_the_house() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans_window(60, 30));
    sit_both(&mut runner);
    detach(&mut runner, 0);
    let clock = clock_of(&runner, 0).expect("the chair is on a clock");
    assert_eq!(clock.what, Deadline::StandIn);
    let out = runner.timeout(clock);
    let session = runner.session().expect("the game is built");
    assert!(
        session
            .seat_kind(PlayerId::new(0))
            .is_some_and(SeatKind::is_away),
        "the house sat down"
    );
    assert!(log_text(&out, 1).contains(r#"{"StandIn":{"player":0}}"#));
}

/// The same race the decision clock has, in the other direction: the
/// socket may come back between the timer firing and this being called,
/// and a player who is at the table must not have their chair taken.
#[test]
fn a_player_who_gets_back_in_time_keeps_their_chair() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans_window(60, 30));
    sit_both(&mut runner);
    detach(&mut runner, 0);
    let clock = clock_of(&runner, 0).expect("the chair is on a clock");
    attach(&mut runner, 0);

    assert!(
        runner.timeout(clock).is_empty(),
        "the deadline fired for a chair that is occupied again"
    );
    assert_eq!(
        clock_of(&runner, 0).map(|c| c.what),
        Some(Deadline::Decide),
        "and the seat is simply being asked again"
    );
}

/// The chair was only ever borrowed, so the socket coming back takes it
/// straight off the house — no window, no second deadline.
#[test]
fn the_house_gives_the_chair_back_when_the_socket_returns() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans_window(60, 30));
    sit_both(&mut runner);
    detach(&mut runner, 0);
    let clock = clock_of(&runner, 0).expect("the chair is on a clock");
    let _ = runner.timeout(clock);
    assert!(
        runner
            .session()
            .expect("the game is built")
            .seat_kind(PlayerId::new(0))
            .is_some_and(SeatKind::is_away),
        "the house is holding seat 0"
    );

    let back = attach(&mut runner, 0);
    assert!(
        !runner
            .session()
            .expect("the game is built")
            .seat_kind(PlayerId::new(0))
            .is_some_and(SeatKind::is_away),
        "the player is back and the chair is theirs"
    );
    assert!(log_text(&back, 1).contains(r#"{"Returned":{"player":0}}"#));
}

/// A deliberate leave with another player at the table: the house takes
/// the chair at once, no window.
#[test]
fn a_player_who_leaves_on_purpose_is_stood_in_for_at_once() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans_window(60, 30));
    sit_both(&mut runner);
    leave(&mut runner, 0);
    let session = runner.session().expect("built");
    assert!(
        session
            .seat_kind(PlayerId::new(0))
            .is_some_and(SeatKind::is_away)
    );
    assert!(!runner.finished(), "another player is at the table");
    assert!(clock_of(&runner, 0).is_none(), "no window for a leave");
}

/// A deliberate leave with nobody else's chair left: the game ends.
#[test]
fn the_last_player_leaving_on_purpose_ends_the_game() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel_window(60, 30));
    sit(&mut runner, 0);
    let out = leave(&mut runner, 0);
    assert!(runner.finished());
    assert!(out.iter().any(
        |e| matches!(&e.msg, Some(v1::envelope::Msg::GameEnded(ended)) if ended.winners == [1])
    ));
}

/// One seat's expired clock must never take another seat's decision, and
/// a deadline that fired after the game moved on must take none at all.
#[test]
fn a_stale_deadline_answers_for_nobody() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel(60));
    sit(&mut runner, 0);
    let clock = on_clock(&runner).expect("someone is being asked");
    let stale = Clock {
        seq: clock.seq + 1,
        ..clock
    };
    assert!(
        runner.timeout(stale).is_empty(),
        "a deadline armed for an older question answered the current one"
    );
    let other_seat = Clock {
        seat: PlayerId::new(1),
        ..clock
    };
    assert!(
        runner.timeout(other_seat).is_empty(),
        "one seat's clock answered for another"
    );
    assert!(
        !runner.timeout(clock).is_empty(),
        "the seat's own deadline did nothing"
    );
}

/// The frames the clock's answer goes out in say the clock gave it. The
/// runner answers through the session's clock door rather than the one
/// a player's answer takes, which is the only way the session can tell.
#[test]
fn the_frames_a_timeout_sends_say_the_clock_answered() {
    use baylee_gamehost::view::wire::HouseAnswer;
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel(60));
    sit(&mut runner, 0);
    let clock = on_clock(&runner).expect("someone is being asked");
    assert_eq!(clock.what, Deadline::Decide);
    let out = runner.timeout(clock);
    let view = last_view(&out, clock.seat.get().into()).expect("the seat is sent the table");
    assert_eq!(
        view.seat(clock.seat).and_then(|s| s.house_answered),
        Some(HouseAnswer::Clock)
    );
}
