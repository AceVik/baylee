//! A seat's first frames, refusals, and the curtain (#256).

use super::*;

/// A seat's first frame has to be the roster and the print table: every
/// frame after it points into them, and without the print table a
/// `PrintRef` names no card at all.
#[test]
fn a_seats_first_frame_is_the_payload_the_rest_refers_to() {
    let mut runner = EngineRunner::new();
    assert!(
        setup(&mut runner, &duel(0)).is_empty(),
        "setup says nothing"
    );
    assert!(runner.ready());
    let out = attach(&mut runner, 0);
    let frames = frames(&out);
    assert!(
        matches!(frames.first(), Some((0, v1::envelope::Msg::GameStatic(_)))),
        "expected the opening payload first, got {:?}",
        frames.first().map(|(seat, _)| seat)
    );
}

/// Frames for a seat that has not arrived are dropped here rather than one
/// hop later, which is what keeps that seat's own opening payload first on
/// its wire when it does arrive.
#[test]
fn a_seat_with_no_socket_is_sent_nothing() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel(0));
    let out = attach(&mut runner, 0);
    assert!(
        frames(&out).iter().all(|(seat, _)| *seat == 0),
        "the absent seat was sent something"
    );
    // And once it arrives, its own attach hands it the whole state.
    let out = attach(&mut runner, 1);
    let frames = frames(&out);
    assert!(matches!(
        frames.first(),
        Some((1, v1::envelope::Msg::GameStatic(_)))
    ));
    assert!(
        frames.len() > 1,
        "a seat that arrives late is left with only a roster"
    );
}

/// A refusal must cost the seat the action and nothing else.
///
/// The network half of the rule `LocalHost` obeys. The client clears its
/// `Interaction` the moment it submits, so an engine that answers a
/// refused action with silence leaves that seat holding no question at
/// all — it can never act again, and the decision clock then hands its
/// turns to the house agent, which from the table looks like the phase
/// advancing on its own.
#[test]
fn a_refused_action_is_answered_with_the_question_again() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel(0));
    sit(&mut runner, 0);
    sit(&mut runner, 1);
    // The opening choice is a mulligan; passing priority is not an answer.
    let out = act(&mut runner, 0, &PlayerAction::PassPriority);
    let refusal = frames(&out);
    assert!(
        refusal
            .iter()
            .any(|(seat, msg)| *seat == 0 && matches!(msg, v1::envelope::Msg::Error(_))),
        "the seat was not told why"
    );
    assert!(
        refusal
            .iter()
            .any(|(seat, msg)| *seat == 0 && matches!(msg, v1::envelope::Msg::ChoiceRequest(_))),
        "the seat was not asked again"
    );
    // One seat's mistake is not everyone's business.
    assert!(
        refusal.iter().all(|(seat, _)| *seat == 0),
        "the refusal was broadcast"
    );
    // And the question that came back is the one it still owes an answer
    // to — proof the re-ask did not advance the game while saying no.
    let out = act(&mut runner, 0, &PlayerAction::MulliganKeep);
    assert!(
        frames(&out)
            .iter()
            .any(|(_, msg)| matches!(msg, v1::envelope::Msg::StateDelta(_))),
        "the seat could not play after the refusal"
    );
}

/// A refusal answers the seat. It must not also buy it time.
///
/// Re-asking made a refused action produce frames where it used to produce
/// none, so it is worth saying outright that the clock does not notice: it
/// is anchored to the sequence number it was armed at, a refusal moves
/// nothing, and the arming loop leaves an identical `Clock` alone. Without
/// that, a seat could hold its own decision open indefinitely by sending
/// illegal actions at it, which is a cheat rather than a bug.
#[test]
fn a_refusal_does_not_restart_the_clock() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel(30));
    sit(&mut runner, 0);
    sit(&mut runner, 1);
    let before = on_clock(&runner).expect("a seat is on the clock");
    let out = act(&mut runner, 0, &PlayerAction::PassPriority);
    assert!(!out.is_empty(), "the refusal was answered at all");
    assert_eq!(
        on_clock(&runner),
        Some(before),
        "a refused action re-armed the clock"
    );
}

/// A human seat's clock starts when the table opens, not when the game
/// is built (#256).
///
/// It used to be the other way round, and this test pinned it before it
/// moved: every human chair owes its opening mulligan from the setup on,
/// so a chair with no socket yet was on its reconnect window from that
/// moment, and a player whose client took longer than the window to load
/// lost the chair to the house before drawing the table. Now nobody is on
/// either clock until the curtain is up, and then the seat that never
/// arrived starts its whole window, and the seat that is here its whole
/// allowance.
#[test]
fn a_seat_with_no_socket_yet_is_waited_for_from_the_curtain_on() {
    let (zero, one) = (PlayerId::new(0), PlayerId::new(1));
    let mut preset = two_humans(600);
    preset.house_rules.reconnect_window_secs = 60;
    let mut runner = EngineRunner::new();
    setup(&mut runner, &preset);
    let kinds = |runner: &EngineRunner| {
        runner
            .clocks()
            .iter()
            .map(|c| (c.seat, c.what, c.secs))
            .collect::<Vec<_>>()
    };
    assert_eq!(kinds(&runner), [], "nobody has attached, and nobody waits");
    sit(&mut runner, 0);
    assert_eq!(
        kinds(&runner),
        [],
        "the first seat in waits for the others with its clock stopped"
    );
    let out = runner.raise_curtain();
    assert_eq!(
        kinds(&runner),
        [(zero, Deadline::Decide, 600), (one, Deadline::StandIn, 60)],
        "the table opened without seat 1, and its window starts now"
    );
    assert!(
        matches!(
            frames(&out).last(),
            Some((0, v1::envelope::Msg::Curtain(_)))
        ),
        "the seat that is here is told the table is open"
    );
}

/// The curtain waits for every human seat, and each is shown its table
/// but asked nothing until the last one is ready (#256). Then every seat
/// is asked, and told the table is open last, after what it opens on.
#[test]
fn the_curtain_goes_up_when_the_last_human_seat_is_ready() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(600));
    // Shown a table: the payload, and a view (after a second payload
    // when the view is the first to show a printing), but no question
    // and no curtain.
    let shown = |out: &[Envelope], seat: u32| {
        let order: Vec<_> = said(out)
            .into_iter()
            .filter(|(_, what)| *what != "loading")
            .collect();
        order.first() == Some(&(seat, "static"))
            && order.last() == Some(&(seat, "view"))
            && order
                .iter()
                .all(|(s, what)| *s == seat && matches!(*what, "static" | "view"))
    };
    let first = attach(&mut runner, 0);
    assert!(shown(&first, 0), "{:?}", said(&first));
    assert_eq!(
        last_view(&first, 0).expect("a view").decision_remaining_ms,
        None,
        "a table that has not opened shows no clock"
    );
    assert!(!ready(&mut runner, 0).is_empty(), "readiness is broadcast");
    assert!(
        runner.entrance_deadline().is_none(),
        "seat 1 is not here yet"
    );
    let second = attach(&mut runner, 1);
    assert!(shown(&second, 1), "{:?}", said(&second));
    assert!(
        said(&second).iter().all(|(s, _)| *s == 1),
        "seat 0's count has not changed, so it is not told it again: {:?}",
        said(&second)
    );
    assert!(runner.curtain_pending());

    let scheduled = ready(&mut runner, 1);
    assert!(
        frames(&scheduled)
            .iter()
            .all(|(_, m)| matches!(m, v1::envelope::Msg::TableLoading(_)))
    );
    assert!(runner.curtain_pending());
    assert!(runner.clocks().is_empty());
    assert!(
        runner.finish_entrance().is_empty(),
        "an early timer cannot open play"
    );
    runner.tell_time(runner.entrance_deadline().unwrap());
    let up = runner.finish_entrance();
    assert!(!runner.curtain_pending());
    for seat in [0, 1] {
        let theirs: Vec<&str> = said(&up)
            .into_iter()
            .filter(|(s, _)| *s == seat)
            .map(|(_, what)| what)
            .collect();
        assert!(
            theirs.contains(&"question"),
            "seat {seat} is asked: {theirs:?}"
        );
        assert_eq!(theirs.last(), Some(&"curtain"), "seat {seat}: {theirs:?}");
    }
    assert_eq!(
        last_view(&up, 0).expect("a view").decision_remaining_ms,
        Some(600_000),
        "the clock starts whole when the table opens"
    );
}

/// An AI chair never holds the curtain, and plays nothing behind it: the
/// house keeps its opening hand in the pump that raises it (#256).
#[test]
fn the_house_plays_nothing_before_the_curtain_is_up() {
    let house = PlayerId::new(1);
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel(600));
    attach(&mut runner, 0);
    let session = runner.session().expect("a game");
    assert!(
        session.awaited().contains(house),
        "the house has decided already"
    );
    let stuck = session.decision_seq();

    ready(&mut runner, 0);
    assert_eq!(
        runner.session().unwrap().decision_seq(),
        stuck,
        "AI waits throughout the portal"
    );
    runner.tell_time(runner.entrance_deadline().unwrap());
    let up = runner.finish_entrance();
    assert_eq!(said(&up).last(), Some(&(0, "curtain")));
    let session = runner.session().expect("a game");
    assert!(!session.awaited().contains(house), "the house did not keep");
    assert!(session.decision_seq() > stuck);
}

#[test]
fn preparation_timeout_never_opens_an_unready_table() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(600));
    attach(&mut runner, 0);
    ready(&mut runner, 0);
    let out = runner.preparation_expired();
    assert!(runner.finished());
    assert!(runner.clocks().is_empty());
    assert!(said(&out).contains(&(0, "error")));
    assert!(!said(&out).iter().any(|(_, what)| *what == "curtain"));
    assert!(
        out.iter()
            .any(|e| matches!(e.msg, Some(v1::envelope::Msg::GameEnded(_))))
    );
}

#[test]
fn a_resynchronized_snapshot_must_earn_readiness_again() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(600));
    attach(&mut runner, 0);
    attach(&mut runner, 1);
    ready(&mut runner, 0);
    ready(&mut runner, 1);
    assert!(runner.entrance_deadline().is_some());
    let refreshed = attach(&mut runner, 0);
    assert!(runner.entrance_deadline().is_none());
    assert!(
        said(&refreshed).contains(&(1, "loading")),
        "the other seat hears its departure cancelled"
    );
    assert!(frames(&refreshed).iter().any(|(_, message)| matches!(message, v1::envelope::Msg::TableLoading(s) if s.ready == 1 && s.enter_at_ms == 0)));
    ready(&mut runner, 0);
    assert!(runner.entrance_deadline().is_some());
}

#[test]
fn a_dropped_seat_cancels_departure_and_must_prepare_again() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(600));
    attach(&mut runner, 0);
    attach(&mut runner, 1);
    ready(&mut runner, 0);
    ready(&mut runner, 1);
    let old_deadline = runner.entrance_deadline().unwrap();
    let cancelled = detach(&mut runner, 1);
    assert!(frames(&cancelled).iter().any(|(_, message)| matches!(message, v1::envelope::Msg::TableLoading(s) if s.ready == 1 && s.enter_at_ms == 0)));
    runner.tell_time(old_deadline);
    assert!(runner.finish_entrance().is_empty());
    assert!(
        ready(&mut runner, 1).is_empty(),
        "a detached seat cannot be ready"
    );
    attach(&mut runner, 1);
    assert!(runner.entrance_deadline().is_none());
    ready(&mut runner, 1);
    assert!(runner.entrance_deadline().unwrap() > old_deadline);
    assert!(runner.clocks().is_empty());
}

/// An action before the curtain is up is dropped without a word: a
/// refusal would reach the client as a failure, and a correct client has
/// been asked nothing it could answer (#256).
#[test]
fn an_action_before_the_curtain_is_dropped_without_a_word() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(600));
    attach(&mut runner, 0);
    let before = runner.session().expect("a game").seq();
    assert!(act(&mut runner, 0, &PlayerAction::MulliganKeep).is_empty());
    assert_eq!(runner.session().expect("a game").seq(), before);
    assert!(
        runner
            .session()
            .expect("a game")
            .awaited()
            .contains(PlayerId::new(0)),
        "the keep went through"
    );
}

/// A seat's `SeatReady` counts for that seat, once, and for nothing
/// after the curtain is up; one from a seat the curtain never waited
/// for, or one beyond any table, counts for nobody (#256).
#[test]
fn a_seat_ready_counts_once_for_its_own_seat() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(600));
    attach(&mut runner, 0);
    attach(&mut runner, 1);
    assert!(!ready(&mut runner, 0).is_empty());
    for _ in 0..3 {
        assert!(ready(&mut runner, 0).is_empty());
    }
    assert!(ready(&mut runner, 7).is_empty(), "no seat 7 at this table");
    assert!(
        ready(&mut runner, 200).is_empty(),
        "nor any seat past a set"
    );
    assert!(runner.curtain_pending(), "seat 1 has not said it");
    assert!(!ready(&mut runner, 1).is_empty());
    assert!(
        ready(&mut runner, 0).is_empty(),
        "the curtain is already up"
    );
}

/// A seat that asks what it missed before the curtain is up is shown its
/// table again, and still asked nothing (#256).
#[test]
fn a_resume_before_the_curtain_asks_nothing() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(600));
    attach(&mut runner, 0);
    let inner = Envelope {
        msg: Some(v1::envelope::Msg::Resume(v1::ResumeGame {
            game_id: "g1".to_string(),
            seat_token: String::new(),
            last_seq: 0,
        })),
    };
    let out = runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                seat: 0,
                envelope: prost::Message::encode_to_vec(&inner).into(),
            })),
        },
        &[],
    );
    assert_eq!(said(&out), [(0, "view")]);
}
