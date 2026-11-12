use super::*;

fn table() -> EngineRunner {
    let mut preset = duel(0);
    preset.seats.push(preset.seats[0].clone());
    for seat in &mut preset.seats {
        seat.controller = baylee_core::preset::SeatController::Open;
    }
    let mut runner = EngineRunner::new();
    setup(&mut runner, &preset);
    for seat in 0..3 {
        sit(&mut runner, seat);
    }
    runner
}

fn ended(out: &[Envelope]) -> Vec<u32> {
    out.iter()
        .filter_map(|env| match &env.msg {
            Some(v1::envelope::Msg::SeatParticipationEnded(ended)) => Some(&ended.seats),
            _ => None,
        })
        .flatten()
        .copied()
        .collect()
}

#[test]
fn concession_releases_only_that_participant_while_others_keep_playing() {
    let mut runner = table();
    let out = act(&mut runner, 0, &PlayerAction::Concede);
    assert_eq!(ended(&out), [0]);
    assert!(!runner.ended, "the other two players still play");
    assert!(runner.session().unwrap().has_left(PlayerId::new(0)));
    assert!(ended(&act(&mut runner, 0, &PlayerAction::Concede)).is_empty());
    assert!(ended(&runner.ending()).is_empty(), "reported only once");
}

#[test]
fn disconnect_and_explicit_leave_with_other_humans_do_not_end_participation() {
    let mut runner = table();
    assert!(ended(&detach(&mut runner, 0)).is_empty());
    assert!(ended(&attach(&mut runner, 0)).is_empty());
    assert!(ended(&leave(&mut runner, 0)).is_empty());
    assert!(!runner.session().unwrap().has_left(PlayerId::new(0)));
    assert!(!runner.ended);
}

fn two_humans_and_house() -> EngineRunner {
    let mut preset = two_humans(0);
    preset.seats.push(duel(0).seats[1].clone());
    let mut runner = EngineRunner::new().with_pump_budget(1);
    setup(&mut runner, &preset);
    sit_both(&mut runner);
    runner
}

#[test]
fn the_last_human_conceding_releases_a_previous_leaver_held_by_the_house() {
    let mut runner = two_humans_and_house();
    assert!(ended(&leave(&mut runner, 0)).is_empty());
    assert!(matches!(
        runner.session().unwrap().seat_kind(PlayerId::new(0)),
        Some(SeatKind::StandIn(_))
    ));
    let out = act(&mut runner, 1, &PlayerAction::Concede);
    assert_eq!(ended(&out), [0, 1]);
    assert!(runner.session().unwrap().has_left(PlayerId::new(1)));
    assert!(!runner.session().unwrap().has_left(PlayerId::new(0)));
    assert!(!runner.ended, "the house may still finish its game");
    assert!(ended(&runner.ending()).is_empty(), "reported only once");
}

#[test]
fn a_disconnected_remaining_human_keeps_the_previous_leavers_participation() {
    let mut runner = two_humans_and_house();
    assert!(ended(&leave(&mut runner, 0)).is_empty());
    assert!(ended(&detach(&mut runner, 1)).is_empty());
    assert!(ended(&runner.ending()).is_empty());
    assert!(!runner.session().unwrap().has_left(PlayerId::new(1)));
}

#[test]
fn a_client_cannot_claim_its_participation_ended() {
    let mut runner = table();
    let forged = Envelope {
        msg: Some(v1::envelope::Msg::SeatParticipationEnded(
            v1::SeatParticipationEnded {
                seats: vec![0, 1, 2],
            },
        )),
    };
    let out = runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                seat: 0,
                envelope: prost::Message::encode_to_vec(&forged).into(),
            })),
        },
        &[],
    );
    assert!(ended(&out).is_empty());
    assert!(!runner.session().unwrap().has_left(PlayerId::new(0)));
}

// ------------------------------------------- the house alone at a table

/// A person in chair 0, then `seats` more chairs: `expert` houses, but chair 1
/// where `bridge` is a seat bridge's (a socket's chair no account sits in).
/// Hosted as an engine hosts a table, the house giving at most `remainder`
/// answers alone; the gateway says which chair an account sits in.
fn a_person_and_experts(slots: &std::path::Path, bridge: bool, remainder: u64) -> EngineRunner {
    use baylee_core::preset::{AIProfile, SeatController};
    let mut preset = duel(0);
    preset.seats.push(preset.seats[1].clone());
    for seat in &mut preset.seats[1..] {
        seat.controller = SeatController::Ai(AIProfile::EXPERT);
    }
    if bridge {
        preset.seats[1].controller = SeatController::Open;
    }
    let mut runner = EngineRunner::new()
        .with_foresight(Some(super::foresight::hosted(slots, &[])))
        .with_remainder_answers(remainder)
        // One house answer a slice, so the bound is read after each.
        .with_pump_budget(1);
    setup_accounts(&mut runner, &preset, vec![true, false, false]);
    if bridge {
        sit_both(&mut runner);
    } else {
        sit(&mut runner, 0);
    }
    runner
}

/// Pumps the house and has `seats` answer as the house would play them,
/// until `done` or none of them is asked.
fn playing(
    runner: &mut EngineRunner,
    seats: &[u32],
    done: impl Fn(&EngineRunner) -> bool,
) -> Vec<Envelope> {
    let mut out = Vec::new();
    for _ in 0..50_000 {
        while runner.needs_pump() {
            out.extend(runner.pump_slice(&[]));
        }
        if runner.ended || done(runner) {
            break;
        }
        let session = runner.session().expect("a game");
        // The seat plays as the house would play it: every answer one the
        // question takes, and a game that goes on as a person's would.
        let Some((seat, answer)) = seats.iter().find_map(|&seat| {
            session
                .house_action(PlayerId::new(seat as u8))
                .map(|answer| (seat, answer))
        }) else {
            break;
        };
        out.extend(act(runner, seat, &answer));
    }
    out
}

fn searched(runner: &EngineRunner) -> u64 {
    runner.session().expect("a game").foresight_stats().searched
}

/// How a game the engine sent `GameEnded` for ended, if it did.
fn game_ended(out: &[Envelope]) -> Option<String> {
    out.iter().find_map(|env| match &env.msg {
        Some(v1::envelope::Msg::GameEnded(ended)) => Some(ended.reason.clone()),
        _ => None,
    })
}

/// When the last person concedes, the gateway frees their account at once,
/// and the houses play the rest of the game out. They do it by their
/// heuristics: the house searches no more, so a game one account starts and
/// concedes holds none of the machine's search slots, however often it does
/// so. The houses finish the game, so its result and its record stand.
#[test]
fn the_house_searches_no_more_once_the_last_person_has_conceded() {
    let dir = crate::foresight::slots_dir();
    let mut runner = a_person_and_experts(dir.path(), false, crate::REMAINDER_ANSWERS);
    playing(&mut runner, &[0], |runner| searched(runner) > 2);
    let before = searched(&runner);
    assert!(before > 2, "the house searched while the person played");
    let decided = runner.session().expect("a game").decision_seq();
    let mut out = act(&mut runner, 0, &PlayerAction::Concede);
    assert_eq!(ended(&out), [0], "the person's account is free at once");
    out.extend(playing(&mut runner, &[0], |_| false));
    assert_eq!(
        searched(&runner),
        before,
        "a search after the last person conceded"
    );
    assert!(runner.ended, "the houses played the game out");
    let reason = game_ended(&out).expect("the game ended");
    assert_ne!(
        reason, "nobody is left",
        "the rules ended it, not the bound"
    );
    let alone = runner.session().expect("a game").decision_seq() - decided;
    assert!(alone < crate::REMAINDER_ANSWERS, "{alone} answers alone");
}

/// A seat bridge its host handed a chair to plays on when its host has
/// conceded, and its host is free to start another game; it is nobody's
/// own chair (`GameSetup.seat_accounts`), so the house does not search for
/// it, and its game is no house-only remainder either: it goes on.
#[test]
fn the_house_does_not_search_for_a_seat_bridge_whose_host_has_gone() {
    let dir = crate::foresight::slots_dir();
    let mut runner = a_person_and_experts(dir.path(), true, 5);
    playing(&mut runner, &[0, 1], |runner| {
        searched(runner) > 2 || runner.session().is_some_and(|s| s.state().turn.number > 8)
    });
    let before = searched(&runner);
    act(&mut runner, 0, &PlayerAction::Concede);
    let turn = runner.session().expect("a game").state().turn.number;
    playing(&mut runner, &[1], |runner| {
        runner
            .session()
            .is_some_and(|s| s.state().turn.number > turn + 4)
    });
    assert_eq!(
        searched(&runner),
        before,
        "a search for a seat bridge alone"
    );
    assert!(
        runner.ended || runner.session().expect("a game").state().turn.number > turn + 4,
        "the bridge's game went on past the house's bound of five answers"
    );
}

/// A game nobody answers over a socket any more that the houses do not
/// finish within their bound is closed as an admin closes one: no winners,
/// and the record ends where it stopped. Here the bound is five answers.
#[test]
fn a_house_only_remainder_is_closed_at_its_bound() {
    let dir = crate::foresight::slots_dir();
    let mut runner = a_person_and_experts(dir.path(), false, 5);
    playing(&mut runner, &[0], |runner| {
        runner.session().is_some_and(|s| s.state().turn.number > 2)
    });
    let mut out = act(&mut runner, 0, &PlayerAction::Concede);
    out.extend(playing(&mut runner, &[0], |_| false));
    assert!(runner.ended);
    assert_eq!(game_ended(&out).as_deref(), Some("nobody is left"));
    let last = out
        .iter()
        .filter_map(|env| match &env.msg {
            Some(v1::envelope::Msg::GameRecordChunk(chunk)) => Some(chunk.last),
            _ => None,
        })
        .next_back();
    assert_eq!(
        last,
        Some(true),
        "the record's last piece went before the end"
    );
}
