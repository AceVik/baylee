//! Two separately constructed sessions take the same two human seats' inputs.
//! This exercises the gamehost, including its per-seat wire frames.

use super::{Envelope, Pending, PlayerAction, PlayerId, SeatController, Session};
use crate::record::{Line, header_of, lines_of};

type Frames = Vec<(PlayerId, Envelope)>;

fn apply(session: &mut Session, line: &Line) -> (u64, Frames) {
    let Line::Input {
        at,
        seat,
        action,
        n,
        hash,
        ..
    } = line
    else {
        panic!("expected an input");
    };
    session.tell_time(*at);
    let frames = session
        .act(PlayerId::new(*seat), action.clone())
        .unwrap_or_else(|error| panic!("input {n}: {error}"));
    for seat in [PlayerId::new(0), PlayerId::new(1)] {
        assert!(
            frames.iter().any(|(recipient, _)| *recipient == seat),
            "seat {seat:?} received no frame for input {n}"
        );
    }
    let snapshot = session.snapshot_hash();
    assert_eq!(
        &crate::record::hex(snapshot),
        hash,
        "recorded state after input {n}"
    );
    (snapshot, frames)
}

fn same_views(a: &Session, b: &Session) {
    assert_eq!(a.decision_seq(), b.decision_seq());
    for seat in [PlayerId::new(0), PlayerId::new(1)] {
        assert_eq!(a.observe_view(seat), b.observe_view(seat));
    }
}

#[test]
fn twin_duels_reproduce_seat_frames_and_the_winner_after_catching_up() {
    let mut lines = lines_of(include_bytes!("../../record/seen/acceptance-seed3.jsonl"));
    let mut preset = header_of(&mut lines).unwrap().preset;
    assert_eq!(preset.seats.len(), 2);
    for seat in &mut preset.seats {
        seat.controller = SeatController::Open;
    }
    preset.house_rules.ranked_duel = true;
    let mut script: Vec<_> = lines.collect::<Result<_, _>>().unwrap();
    let Some(Line::End {
        winners,
        reason,
        turn,
        ..
    }) = script.pop()
    else {
        panic!("the fixture must include the finish");
    };
    // The original game delegated decisions to the house. Both socket seats
    // here submit those same choices themselves, so no chair changes hands.
    script.retain(|line| !matches!(line, Line::Chair { .. } | Line::DeclaredMind { .. }));

    let mut lead = Session::new(&preset).unwrap();
    let mut twin = Session::new(&preset).unwrap();
    same_views(&lead, &twin);
    assert_eq!(lead.snapshot_hash(), twin.snapshot_hash());

    // A legal alternate decision in the very same opening must be detectable.
    let mut different = Session::new(&preset).unwrap();
    let Line::Input {
        seat,
        at,
        action: PlayerAction::MulliganKeep,
        ..
    } = &script[0]
    else {
        panic!("expected the recorded keep");
    };
    different.tell_time(*at);
    different
        .act(PlayerId::new(*seat), PlayerAction::MulliganTake)
        .unwrap();

    let (prefix, rest) = script.split_at(32);
    for (index, line) in prefix.iter().enumerate() {
        assert_eq!(apply(&mut lead, line), apply(&mut twin, line));
        if index == 0 {
            assert_ne!(lead.snapshot_hash(), different.snapshot_hash());
        }
    }

    // The replica misses sixteen consecutive inputs while the leader plays.
    // Supply the missing inputs in order and check each intermediate response.
    let (missing, suffix) = rest.split_at(16);
    let paused = twin.snapshot_hash();
    let expected: Vec<_> = missing.iter().map(|line| apply(&mut lead, line)).collect();
    assert_eq!(twin.snapshot_hash(), paused);
    assert_ne!(lead.snapshot_hash(), paused);
    for (line, observed) in missing.iter().zip(expected) {
        assert_eq!(apply(&mut twin, line), observed);
    }
    same_views(&lead, &twin);
    for line in suffix {
        assert_eq!(apply(&mut lead, line), apply(&mut twin, line));
    }

    same_views(&lead, &twin);
    for session in [&lead, &twin] {
        let Pending::GameOver(result) = session.pending() else {
            panic!("the actual engine did not reach a terminal state");
        };
        let actual: Vec<_> = session
            .winning_seats(*result)
            .iter()
            .map(|p| p.get())
            .collect();
        assert_eq!(actual, winners);
        assert_eq!(format!("{:?}", result.reason), reason);
        assert_eq!(Some(session.state().turn.number), turn);
    }
}
