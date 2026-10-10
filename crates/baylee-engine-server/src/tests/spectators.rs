use super::*;

// ---- spectators (`docs/protocol.md` §"Spectators")

fn spectators(runner: &mut EngineRunner, count: u32, joined: bool) -> Vec<Envelope> {
    runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::SpectatorsChanged(
                v1::SpectatorsChanged { count, joined },
            )),
        },
        &[],
    )
}

/// The inner messages of every spectator frame.
fn watched(out: &[Envelope]) -> Vec<v1::envelope::Msg> {
    out.iter()
        .filter_map(|env| match &env.msg {
            Some(v1::envelope::Msg::SpectatorFrame(frame)) => {
                <Envelope as prost::Message>::decode(&frame.envelope[..])
                    .ok()?
                    .msg
            }
            _ => None,
        })
        .collect()
}

/// A spectator who arrives before the curtain is sent the public table and
/// an open curtain at once, and the curtain still waits for the seats and
/// only for them.
#[test]
fn a_spectator_is_never_waited_for() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(600));
    let out = spectators(&mut runner, 1, true);
    let msgs = watched(&out);
    assert!(matches!(
        msgs.first(),
        Some(v1::envelope::Msg::GameStatic(_))
    ));
    assert!(
        msgs.iter()
            .any(|m| matches!(m, v1::envelope::Msg::StateDelta(_)))
    );
    assert!(matches!(msgs.last(), Some(v1::envelope::Msg::Curtain(_))));
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, v1::envelope::Msg::ChoiceRequest(_))),
        "a spectator is asked nothing"
    );
    assert!(
        runner.curtain_pending(),
        "a spectator does not open the table"
    );
    sit(&mut runner, 0);
    let up = sit(&mut runner, 1);
    assert!(!runner.curtain_pending(), "the seats alone open it");
    assert!(
        !watched(&up).is_empty(),
        "the pump that opens the table tells the spectators"
    );
    assert!(runner.clocks().len() <= 2, "no clock is a spectator's");
}

/// Every attached seat is told how many watch, and a seat that attaches
/// later is told the number with its table.
#[test]
fn seats_are_told_how_many_watch() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(600));
    sit(&mut runner, 0);
    let out = spectators(&mut runner, 2, true);
    assert!(frames(&out).iter().any(
        |(seat, m)| *seat == 0 && matches!(m, v1::envelope::Msg::Spectators(s) if s.count == 2)
    ));
    let later = attach(&mut runner, 1);
    assert!(frames(&later).iter().any(
        |(seat, m)| *seat == 1 && matches!(m, v1::envelope::Msg::Spectators(s) if s.count == 2)
    ));
}

/// An action in no seat's name, as a spectator's would be if anything ever
/// forwarded one, moves nothing and is answered to nobody.
#[test]
fn an_action_from_no_seat_is_refused() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(600));
    sit(&mut runner, 0);
    sit(&mut runner, 1);
    spectators(&mut runner, 1, true);
    let seq = runner.session().expect("a game").seq();
    let spectator = u32::from(baylee_gamehost::view::SPECTATOR.get());
    let out = act(&mut runner, spectator, &PlayerAction::MulliganKeep);
    assert_eq!(
        runner.session().expect("a game").seq(),
        seq,
        "nothing moved"
    );
    assert!(frames(&out).is_empty(), "{:?}", said(&out));
    assert!(watched(&out).is_empty());
}
