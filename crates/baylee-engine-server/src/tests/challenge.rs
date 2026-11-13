//! Real frozen games through the same runner as the network process.
use super::*;
use sha2::{Digest, Sha256};

fn frozen() -> GamePreset {
    let mut preset = duel(0);
    preset.house_rules.reconnect_window_secs = 0;
    preset.house_rules.takebacks = false;
    preset.house_rules.time_extension_votes = false;
    preset.house_rules.challenge_policy_seed = Some(74);
    preset.seats[1].controller =
        baylee_core::preset::SeatController::Ai(baylee_core::preset::AIProfile {
            foresight: 0,
            ..baylee_core::preset::AIProfile::STEADY
        });
    preset
}

fn result(frames: &[Envelope]) -> &v1::GameEnded {
    frames
        .iter()
        .find_map(|frame| match &frame.msg {
            Some(v1::envelope::Msg::GameEnded(ended)) => Some(ended),
            _ => None,
        })
        .expect("one authoritative ending")
}

#[test]
fn continuous_human_time_and_streamed_record_are_bound_to_the_real_result() {
    let mut runner = EngineRunner::new();
    runner.tell_time(1_800_000_000_000);
    let mut frames = setup(&mut runner, &frozen());
    runner.wait_from(0);
    runner.measure_until(90_000); // Loading never counts.
    frames.extend(sit(&mut runner, 0));
    let mut elapsed = 100_000;
    let mut decisions = 0;
    while !runner.finished() && decisions < 5000 {
        while runner.needs_pump() {
            frames.extend(runner.pump_slice(&[]));
        }
        if runner.finished() {
            break;
        }
        runner.wait_from(elapsed);
        elapsed += 250;
        runner.measure_until(elapsed);
        let action = runner
            .session()
            .unwrap()
            .house_action(PlayerId::new(0))
            .unwrap();
        frames.extend(act(&mut runner, 0, &action));
        decisions += 1;
        elapsed += 100; // Engine computation is excluded.
    }
    assert!(runner.finished());
    let end = result(&frames);
    let evidence = end.challenge.as_ref().unwrap();
    assert_eq!(evidence.human_ms, Some(decisions * 250));
    assert_eq!(evidence.finished_at_unix_ms, runner.now);
    let compressed: Vec<u8> = frames
        .iter()
        .filter_map(|frame| match &frame.msg {
            Some(v1::envelope::Msg::GameRecordChunk(piece)) => Some(piece.data.as_slice()),
            _ => None,
        })
        .flatten()
        .copied()
        .collect();
    let mut record = Vec::new();
    std::io::Read::read_to_end(
        &mut flate2::read::MultiGzDecoder::new(&compressed[..]),
        &mut record,
    )
    .unwrap();
    assert_eq!(evidence.record_sha256, Sha256::digest(&record).as_slice());
    assert!(baylee_gamehost::record::replay(&record).unwrap().ended);
    assert!(matches!(
        v1::challenge_result::Outcome::try_from(evidence.outcome).unwrap(),
        v1::challenge_result::Outcome::Won | v1::challenge_result::Outcome::Lost
    ));
}

#[test]
fn disconnect_resumes_without_a_failure_but_excludes_sprint_and_hold_expiry_is_void() {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &frozen());
    sit(&mut runner, 0);
    let decisions = runner.session().unwrap().decision_seq();
    let dropped = detach(&mut runner, 0);
    assert!(!runner.finished());
    assert!(
        dropped
            .iter()
            .all(|frame| !matches!(frame.msg, Some(v1::envelope::Msg::GameEnded(_))))
    );
    attach(&mut runner, 0);
    assert_eq!(runner.session().unwrap().decision_seq(), decisions);
    assert_eq!(
        runner
            .challenge_result(v1::challenge_result::Outcome::Won)
            .unwrap()
            .human_ms,
        None
    );
    detach(&mut runner, 0);
    let frames = runner.timeout(Clock {
        seat: PlayerId::new(0),
        what: Deadline::Hold,
        seq: 0,
        secs: HOLD_SECS,
    });
    assert_eq!(
        result(&frames).challenge.as_ref().unwrap().outcome,
        v1::challenge_result::Outcome::Void as i32
    );
}

#[test]
fn deliberate_leave_is_abandoned_even_during_loading() {
    for loaded in [false, true] {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &frozen());
        if loaded {
            sit(&mut runner, 0);
        } else {
            attach(&mut runner, 0);
        }
        let frames = runner.handle(
            Envelope {
                msg: Some(v1::envelope::Msg::SeatDetached(v1::SeatDetached {
                    seat: 0,
                    left: true,
                })),
            },
            &[],
        );
        assert_eq!(
            result(&frames).challenge.as_ref().unwrap().outcome,
            v1::challenge_result::Outcome::Abandoned as i32
        );
        assert!(result(&frames).turn > 0);
    }
}
