//! An admin ending a game (`docs/protocol.md` §"Acting on a game"):
//! `CloseGame` tells every seat why, ends the record and the game.

use super::*;

fn close(runner: &mut EngineRunner, reason: &str) -> Vec<Envelope> {
    runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::CloseGame(v1::CloseGame {
                game_id: "g1".to_string(),
                reason: reason.to_string(),
            })),
        },
        &[],
    )
}

fn ended_with(out: &[Envelope]) -> Vec<&v1::GameEnded> {
    out.iter()
        .filter_map(|e| match &e.msg {
            Some(v1::envelope::Msg::GameEnded(ended)) => Some(ended),
            _ => None,
        })
        .collect()
}

#[test]
fn closing_a_game_tells_each_seat_why_and_ends_it_after_the_record() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(600));
    sit(&mut runner, 0);
    sit(&mut runner, 1);
    assert!(!runner.finished());

    let out = close(&mut runner, "a rule dispute");
    assert!(runner.finished(), "the process may exit");

    let told: Vec<u32> = frames(&out)
        .into_iter()
        .filter_map(|(seat, msg)| match msg {
            v1::envelope::Msg::GameClosed(closed) => {
                assert_eq!(closed.reason, "a rule dispute");
                Some(seat)
            }
            _ => None,
        })
        .collect();
    assert_eq!(told, [0, 1], "every attached seat is told, and only once");

    // No spectators here, so none is told.
    assert!(
        !out.iter()
            .any(|e| matches!(e.msg, Some(v1::envelope::Msg::SpectatorFrame(_)))),
        "nobody watches"
    );

    // The record ends where the game was closed, and only then is the
    // game over: a gateway that has read `GameEnded` has read it all.
    let kinds: Vec<&str> = out
        .iter()
        .map(|e| match &e.msg {
            Some(v1::envelope::Msg::SeatFrame(_)) => "seat",
            Some(v1::envelope::Msg::GameRecordChunk(chunk)) => {
                assert!(chunk.last, "the one piece left is the last");
                assert_eq!(chunk.game_id, "g1");
                "record"
            }
            Some(v1::envelope::Msg::GameEnded(_)) => "ended",
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(kinds, ["seat", "seat", "record", "ended"]);
    let ended = ended_with(&out);
    assert_eq!(ended.len(), 1);
    assert_eq!(ended[0].game_id, "g1");
    assert!(ended[0].winners.is_empty(), "nobody won: {:?}", ended[0]);
}

#[test]
fn a_game_is_closed_once() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(600));
    sit(&mut runner, 0);
    sit(&mut runner, 1);
    assert!(!close(&mut runner, "first").is_empty());
    assert!(
        close(&mut runner, "second").is_empty(),
        "an ended game is left alone"
    );
    assert!(runner.finished());
}

#[test]
fn a_seat_that_has_left_is_not_told_and_a_spectator_is() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(600));
    sit(&mut runner, 0);
    sit(&mut runner, 1);
    detach(&mut runner, 1);
    runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::SpectatorsChanged(
                v1::SpectatorsChanged {
                    count: 1,
                    joined: true,
                },
            )),
        },
        &[],
    );
    let out = close(&mut runner, "closing time");
    let told: Vec<u32> = frames(&out)
        .into_iter()
        .filter(|(_, msg)| matches!(msg, v1::envelope::Msg::GameClosed(_)))
        .map(|(seat, _)| seat)
        .collect();
    assert_eq!(told, [0], "only the seat that is at the table");
    let watched: Vec<v1::envelope::Msg> = out
        .iter()
        .filter_map(|e| match &e.msg {
            Some(v1::envelope::Msg::SpectatorFrame(frame)) => {
                <Envelope as prost::Message>::decode(&frame.envelope[..])
                    .ok()?
                    .msg
            }
            _ => None,
        })
        .collect();
    assert!(
        matches!(watched.as_slice(), [v1::envelope::Msg::GameClosed(c)] if c.reason == "closing time"),
        "{watched:?}"
    );
}
