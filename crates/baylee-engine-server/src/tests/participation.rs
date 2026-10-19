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
