//! Building the game once, what a view says about time, seat settings and
//! reasoning.

use super::*;

/// Everything before the game exists is ignored rather than acted on: a
/// seat frame that arrives first is a race, not an attack, and neither
/// deserves a panic.
#[test]
fn frames_before_the_game_exists_do_nothing() {
    let mut runner = EngineRunner::new();
    assert!(attach(&mut runner, 0).is_empty());
    assert!(
        runner
            .handle(
                Envelope {
                    msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                        seat: 0,
                        envelope: bytes::Bytes::new(),
                    })),
                },
                &[]
            )
            .is_empty()
    );
    assert!(!runner.finished());
}

/// A second `GameSetup` must not rebuild a game that is already being
/// played — the seats would silently be handed a different one.
#[test]
fn a_game_is_built_once() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel(0));
    let seq_before = runner.session().expect("a game").seq();
    assert!(setup(&mut runner, &duel(0)).is_empty());
    assert_eq!(
        runner.session().expect("still the same game").seq(),
        seq_before
    );
}

/// A question nobody has spent time on is worth the whole allowance, and
/// `blitz` is what proves the threshold is flat: at thirty seconds the
/// number is on from the very first question rather than appearing part
/// way through.
#[test]
fn a_question_just_asked_is_shown_the_whole_allowance() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel(30));
    let out = sit(&mut runner, 0);
    assert_eq!(
        last_view(&out, 0)
            .expect("seat 0 was sent a view")
            .decision_remaining_ms,
        Some(30_000),
        "the opening question of a blitz table showed no clock"
    );
}

/// The reading the caller took is what the seat is shown — the case the
/// parameter exists for, and the one a reconnecting player depends on.
#[test]
fn a_reading_taken_for_this_question_is_what_the_seat_is_shown() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel(600));
    sit(&mut runner, 0);
    // A socket returning mid-question: the game has not moved, so this
    // is the same question with four seconds left on it.
    let out = runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::SeatAttached(v1::SeatAttached {
                seat: 0,
                resync: true,
            })),
        },
        &reading(&runner, 4_000),
    );
    assert_eq!(
        last_view(&out, 0)
            .expect("seat 0 was sent a view")
            .decision_remaining_ms,
        Some(4_000),
        "a seat that reconnected was shown the allowance it started with, \
         not the time it has left"
    );
}

/// The anchor check, and the one that will silently regress if someone
/// later simplifies the `decision_seq` comparison away: once the question
/// has moved, the previous question's leftovers must not run against it.
#[test]
fn a_view_built_after_the_question_moved_shows_the_whole_allowance() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel(600));
    sit(&mut runner, 0);
    let before = runner.session().expect("a game").decision_seq();
    // Four seconds left on *this* question, and then an answer that
    // moves the game on to the next one. The reading has to be a partial
    // one or the test proves nothing: with the whole allowance on both
    // sides, a stale value and a correct one are the same number.
    let inner = Envelope {
        msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
            game_id: "g1".to_string(),
            seat_token: String::new(),
            action_json: serde_json::to_vec(&PlayerAction::MulliganKeep)
                .expect("action serializes"),
        })),
    };
    let out = runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                seat: 0,
                envelope: prost::Message::encode_to_vec(&inner).into(),
            })),
        },
        &reading(&runner, 4_000),
    );
    assert_ne!(
        runner.session().expect("a game").decision_seq(),
        before,
        "this test needs an action that actually asks a new question"
    );
    let seen = last_view(&out, 0).expect("seat 0 was sent a view");
    assert_eq!(
        seen.decision_remaining_ms,
        Some(600_000),
        "the new question inherited the old one's remainder"
    );
}

/// An untimed table has no number, and `0` would be a seat with no time
/// left rather than a seat with no limit.
#[test]
fn an_untimed_table_shows_no_number_at_all() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel(0));
    let out = attach(&mut runner, 0);
    assert_eq!(
        last_view(&out, 0)
            .expect("seat 0 was sent a view")
            .decision_remaining_ms,
        None,
        "an untimed table drew a countdown"
    );
}

/// A seat whose socket is gone is on the stand-in clock, which is not a
/// decision clock: the other seat must not be shown a countdown against
/// a player who is not deciding.
#[test]
fn a_seat_waiting_out_its_reconnect_window_is_on_no_decision_clock() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(600));
    sit(&mut runner, 0);
    sit(&mut runner, 1);
    // Past the mulligans, which ask both seats at once: this is about
    // the seat that is not being asked.
    keep_both(&mut runner);
    detach(&mut runner, 0);
    // Seat 1 is still here and still being sent views; seat 0, which the
    // table is waiting on, is on the reconnect window instead.
    let out = sit(&mut runner, 1);
    assert_eq!(
        on_clock(&runner).map(|c| c.what),
        Some(Deadline::StandIn),
        "this test needs seat 0 to be on the reconnect window"
    );
    assert_eq!(
        last_view(&out, 1)
            .expect("seat 1 was sent a view")
            .decision_remaining_ms,
        None,
        "a seat with no socket was drawn a decision countdown"
    );
}

/// The clock is public: the seat that is *not* being asked is told how
/// long the seat that is has left. A table where one player is running
/// out of time and nobody else can see it reads the pause as rudeness.
///
/// From turn 1 on, so past the mulligans, where every seat is asked its
/// own question and told its own remainder (the next test).
#[test]
fn the_other_seat_is_told_the_awaited_seats_remainder() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(600));
    sit(&mut runner, 0);
    sit(&mut runner, 1);
    act(&mut runner, 0, &PlayerAction::MulliganKeep);
    let out = act(&mut runner, 1, &PlayerAction::MulliganKeep);
    let awaited = runner.session().expect("a game").awaiting_seat();
    assert_eq!(
        awaited.map(PlayerId::get),
        Some(0),
        "this test needs seat 0 to be the one being asked"
    );
    assert_eq!(
        last_view(&out, 1)
            .expect("seat 1 was sent a view")
            .decision_remaining_ms,
        Some(600_000),
        "seat 1 was not told the clock seat 0 is on"
    );
}

/// During the opening mulligans each deciding seat is on its own clock
/// and is told its own remainder. A seat that has kept is told none: its
/// view waits on nobody (`PlayerView::awaiting`), so a number there
/// would name nobody's clock.
#[test]
fn in_the_mulligans_each_seat_is_told_its_own_remainder() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &two_humans(600));
    sit(&mut runner, 0);
    sit(&mut runner, 1);
    let [zero, one] = runner.clocks()[..] else {
        panic!("both seats are deciding");
    };
    // Seat 0 takes with nine seconds left and seat 1 has four: seat 0 is
    // asked anew, and seat 1 is still on the question it had.
    let out = act_reading(
        &mut runner,
        0,
        &PlayerAction::MulliganTake,
        &[(zero, 9_000), (one, 4_000)],
    );
    let told = |out: &[Envelope], seat: u32| {
        let view = last_view(out, seat).expect("a view");
        (view.awaiting.map(PlayerId::get), view.decision_remaining_ms)
    };
    assert_eq!(told(&out, 0), (Some(0), Some(600_000)));
    assert_eq!(told(&out, 1), (Some(1), Some(4_000)));

    // Seat 1 keeps with nine seconds gone from seat 0's new question,
    // which is still the question seat 0 owes: the reading stays with it.
    let [zero, one] = runner.clocks()[..] else {
        panic!("both seats are deciding");
    };
    let out = act_reading(
        &mut runner,
        1,
        &PlayerAction::MulliganKeep,
        &[(zero, 591_000), (one, 3_000)],
    );
    assert_eq!(told(&out, 1), (None, None), "seat 1 has kept");
    assert_eq!(told(&out, 0), (Some(0), Some(591_000)));
    let deciding = last_view(&out, 1).expect("a view").deciding;
    assert_eq!(deciding.iter().map(PlayerId::get).collect::<Vec<_>>(), [0]);
}

/// Two players on one team, against the house.
fn partners() -> GamePreset {
    let mut preset = two_humans(600);
    let mut house = preset.seats[1].clone();
    house.controller =
        baylee_core::preset::SeatController::Ai(baylee_core::preset::AIProfile::default());
    house.team = Some(2);
    preset.seats[0].team = Some(1);
    preset.seats[1].team = Some(1);
    preset.seats.push(house);
    preset
}

/// A seat's setting is heard before the curtain is up as well as after
/// it (#265): it is no decision and runs no clock. Either way it sends
/// the views it changed and asks nothing, and a setting that is refused
/// is answered to the seat that sent it and to nobody else.
#[test]
fn a_seat_setting_is_heard_before_the_curtain_and_after_and_asks_nothing() {
    let only_views = |out: &[Envelope]| {
        !out.is_empty()
            && said(out)
                .iter()
                .all(|(_, what)| matches!(*what, "static" | "view"))
    };
    let hands = |out: &[Envelope], seat: u32| {
        last_view(out, seat)
            .expect("the teammate is sent a view")
            .shared_hands
            .iter()
            .map(|h| h.player.get())
            .collect::<Vec<_>>()
    };
    let mut runner = EngineRunner::new();
    setup(&mut runner, &partners());
    attach(&mut runner, 0);
    attach(&mut runner, 1);
    assert!(runner.curtain_pending());
    let partner: baylee_core::ids::SeatSet = [PlayerId::new(1)].into_iter().collect();

    let shown = set(&mut runner, 0, SeatSetting::ShareHand(partner));
    assert!(only_views(&shown), "{:?}", said(&shown));
    assert_eq!(hands(&shown, 1), [0]);

    ready(&mut runner, 0);
    ready(&mut runner, 1);
    runner.tell_time(runner.entrance_deadline().unwrap());
    runner.finish_entrance();
    assert!(!runner.curtain_pending());
    let withdrawn = set(
        &mut runner,
        0,
        SeatSetting::ShareHand(baylee_core::ids::SeatSet::new()),
    );
    assert!(only_views(&withdrawn), "{:?}", said(&withdrawn));
    assert_eq!(hands(&withdrawn, 1), [0; 0]);

    let house: baylee_core::ids::SeatSet = [PlayerId::new(2)].into_iter().collect();
    let refused = set(&mut runner, 0, SeatSetting::ShareHand(house));
    assert_eq!(said(&refused), [(0, "error")]);
}

/// What a seat's mind said, wrapped the way its socket sends it.
fn reasoning_from(runner: &mut EngineRunner, seat: u32, claimed: u32) -> Vec<Envelope> {
    let inner = Envelope {
        msg: Some(v1::envelope::Msg::AiLog(v1::AiLog {
            note: r#"{"chose":"a1 Pass"}"#.to_string(),
            thinking: "my hand is two Islands and a Counterspell".to_string(),
            seat: claimed,
        })),
    };
    runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                seat,
                envelope: prost::Message::encode_to_vec(&inner).into(),
            })),
        },
        &[],
    )
}

/// Every `AiLog` in `out`: the seat it goes to, and the seat it names.
fn reasonings(out: &[Envelope]) -> Vec<(u32, u32)> {
    frames(out)
        .into_iter()
        .filter_map(|(to, msg)| match msg {
            v1::envelope::Msg::AiLog(said) => Some((to, said.seat)),
            _ => None,
        })
        .collect()
}

/// Three attached seats, two of them a team and the third the other
/// side, each with a socket, so every seat is one that could be sent a
/// reasoning.
fn three_at_the_table() -> EngineRunner {
    let mut preset = partners();
    preset.seats[2].controller = baylee_core::preset::SeatController::Open;
    let mut runner = EngineRunner::new();
    setup(&mut runner, &preset);
    for seat in 0..3 {
        sit(&mut runner, seat);
    }
    runner
}

/// In a debug build the AI log is open to the table (the owner's
/// decision, `docs/protocol.md` §"An AI seat's reasoning"): a seat's
/// reasoning reaches every other attached seat, the other side and a
/// teammate not shown the hand included, never back to the sender, and
/// it names the seat that really sent it, whatever the sender wrote.
#[cfg(debug_assertions)]
#[test]
fn in_a_debug_build_a_seats_reasoning_reaches_every_other_seat_in_its_name() {
    let mut runner = three_at_the_table();
    let claimed = reasoning_from(&mut runner, 0, 2);
    assert_eq!(
        reasonings(&claimed),
        [(1, 0), (2, 0)],
        "both other seats, told it is seat 0's though it claimed seat 2"
    );
    let opponent = reasoning_from(&mut runner, 2, 0);
    assert_eq!(reasonings(&opponent), [(0, 2), (1, 2)]);
}

/// In a release build, the one deployed and the one CI's `test-release`
/// runs, nobody is sent a seat's reasoning: the engine drops it,
/// whoever sent it and whatever the table shares.
#[cfg(not(debug_assertions))]
#[test]
fn in_a_release_build_nobody_is_sent_a_seats_reasoning() {
    let mut runner = three_at_the_table();
    let partner: baylee_core::ids::SeatSet = [PlayerId::new(1)].into_iter().collect();
    set(&mut runner, 0, SeatSetting::ShareHand(partner));
    for (from, claimed) in [(0, 0), (0, 2), (1, 1), (2, 2)] {
        let out = reasoning_from(&mut runner, from, claimed);
        assert_eq!(reasonings(&out), [], "seat {from}'s reasoning went out");
        assert_eq!(said(&out), [], "seat {from}'s reasoning sent something");
    }
}
