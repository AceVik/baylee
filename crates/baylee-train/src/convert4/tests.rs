use super::*;
use crate::housedeck::HouseDeck;
use crate::selfplay::{Caps, play_with_options, table};
use baylee_core::preset::AIProfile;
use std::time::Duration;

#[test]
fn streamed_frames_match_collecting_conversion_with_bounded_memory() {
    let a = HouseDeck::named("allytifact").unwrap();
    let b = HouseDeck::named("victory").unwrap();
    for (decks, teams) in [
        (vec![&a, &b], vec![]),
        (vec![&a, &b, &a, &b], vec![1, 2, 1, 2]),
    ] {
        let preset =
            crate::selfplay::table_for(19, &decks, &vec![AIProfile::SHARP; decks.len()], &teams);
        let played = play_with_options(
            &preset,
            "stream-equality",
            Caps {
                answers: 10_000,
                wall: Duration::from_secs(60),
            },
            true,
        );
        assert!(played.outcome.finished());
        let original = extract_game4(&played.record, 17, true).unwrap();
        let mut streamed = Vec::new();
        let result = stream_game4(&played.record, 17, true, None, |frame| {
            streamed.push(frame);
            Ok(())
        })
        .unwrap();
        assert!(result.peak_buffered_frames <= decks.len());
        assert_eq!(result.steps as usize, original.steps.len());
        assert_eq!(result.coverage.counts, original.coverage.counts);
        assert_eq!(result.unencoded_decisions, original.unencoded_decisions);
        let ordered = |mut frames: Vec<Step4>| {
            frames.sort_by_key(|s| (s.seat, s.step_cursor));
            frames
                .into_iter()
                .map(|s| serde_json::to_value(s).unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(ordered(streamed), ordered(original.steps));
        let mut payment_frames = Vec::new();
        let payment_result = stream_game4_with_encoder(
            &played.record,
            17,
            true,
            None,
            None,
            None,
            false,
            8,
            |frame| {
                payment_frames.push(frame);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(payment_result.unencoded_decisions, 0);
        assert!(!payment_frames.is_empty());
        assert!(payment_result.peak_buffered_frames <= decks.len());
        for frame in &payment_frames {
            assert!(
                frame
                    .state
                    .entity_feat
                    .iter()
                    .all(|row| row.len() == crate::features7::ENTITY_FEAT_WIDTH7)
            );
            assert_eq!(
                frame.state.global_feat.len(),
                crate::features7::GLOBAL_FEAT_WIDTH7
            );
            assert!((0.0..=1.0).contains(&frame.outcome));
        }

        // No corrupted input reaches a consumer, even if the recorded action
        // would still be legal: the audit must check hashes, not only legality.
        let mut lines = played
            .record
            .split(|&b| b == b'\n')
            .filter(|s| !s.is_empty())
            .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        lines[0]["hash"] = serde_json::json!("0000000000000000");
        lines[0]["seen"] = serde_json::json!("0000000000000000");
        let mut bad = Vec::new();
        for line in &lines {
            serde_json::to_writer(&mut bad, line).unwrap();
            bad.push(b'\n');
        }
        let mut emitted = 0;
        assert!(
            stream_game4(&bad, 17, true, None, |_| {
                emitted += 1;
                Ok(())
            })
            .is_err()
        );
        assert_eq!(emitted, 0);
        assert!(
            stream_game4(&played.record, 17, true, None, |_| Err(
                "test output failure".into()
            ))
            .unwrap_err()
            .contains("test output failure")
        );
    }
}

#[test]
fn extract_game4_replays_fair_selfplay_with_zero_unencoded_decisions() {
    let a = HouseDeck::named("allytifact").unwrap();
    let b = HouseDeck::named("victory").unwrap();
    let preset = table(19, &a, &b, [AIProfile::SHARP, AIProfile::EXPERT]);
    let played = play_with_options(
        &preset,
        "t-convert4-19",
        Caps {
            answers: 10_000,
            wall: Duration::from_secs(60),
        },
        true,
    );
    assert!(played.outcome.finished(), "{:?}", played.outcome);
    let extracted = extract_game4(&played.record, 1, false).expect("extracts cleanly");
    assert!(!extracted.steps.is_empty());
    assert_eq!(
        extracted.unencoded_decisions, 0,
        "every non-forced decision must encode into a v4 target"
    );
    for st in &extracted.steps {
        assert!((0.0..=1.0).contains(&st.outcome));
        assert!(!st.state.entity_feat.is_empty());
    }
    // Verify PassPriority steps are retained in trajectory order (with 0.2 weight on 5/6)
    // and final step of each seat trajectory has is_terminal = true
    let mut last_per_seat = std::collections::BTreeMap::new();
    for (idx, st) in extracted.steps.iter().enumerate() {
        last_per_seat.insert(st.seat, idx);
    }
    for (&_seat, &last_idx) in &last_per_seat {
        assert!(
            extracted.steps[last_idx].is_terminal,
            "final step of seat trajectory must have is_terminal = true"
        );
    }
}

#[test]
fn observe_view_hides_opponent_private_hand_and_library() {
    use crate::selfplay::table_for;
    use baylee_core::ids::PlayerId;
    use baylee_gamehost::Session;

    let d1 = HouseDeck::named("allytifact").unwrap();
    let d2 = HouseDeck::named("victory").unwrap();
    let preset = table_for(99, &[&d1, &d2], &[AIProfile::STEADY; 2], &[]);
    let session = Session::new(&preset).unwrap();
    let v0 = session.observe_view(PlayerId::new(0));
    let v1 = session.observe_view(PlayerId::new(1));
    assert_eq!(v0.decision_player, Some(PlayerId::new(0)));
    assert_eq!(v1.decision_player, Some(PlayerId::new(1)));
    assert!(!v0.hand.is_empty());
    assert!(!v1.hand.is_empty());
    // Seat 0's hand card ObjectIds must not appear in Seat 1's hand
    for card in &v0.hand {
        assert!(!v1.hand.iter().any(|c| c.id == card.id));
    }
}

#[test]
fn unsupported_encoder_rejects_before_any_frames_are_emitted() {
    let mut emitted = 0;
    let result = stream_game4_with_encoder(b"", 1, true, None, None, None, false, 10, |_| {
        emitted += 1;
        Ok(())
    });
    assert!(result.unwrap_err().contains("Unsupported encoder"));
    assert_eq!(emitted, 0);
}

#[test]
fn encoder9_sidecars_require_matching_explicit_producer_width() {
    let mut run = serde_json::json!({"model_identity":{"encoder_version":9,"model_version":5,
        "architecture":"baylee_ai_v5_relational_v2","global_feat_width":82}});
    assert!(ProducerEncoder::from_run(&run).is_ok());
    run["model_identity"]["global_feat_width"] = serde_json::json!(80);
    assert!(ProducerEncoder::from_run(&run).is_err());
    let error = stream_game4_with_encoder(b"", 1, true, Some(&[]), None, None, false, 9, |_| {
        panic!("no emission")
    })
    .unwrap_err();
    assert!(error.contains("verified producer encoder 9"));
}

#[test]
fn encoder8_sidecars_require_matching_verified_producer_before_replay() {
    let mut emitted = 0;
    let unbound = stream_game4_with_encoder(b"", 1, true, Some(&[]), None, None, false, 8, |_| {
        emitted += 1;
        Ok(())
    });
    assert!(unbound.unwrap_err().contains("verified producer encoder 8"));
    let producer7 = ProducerEncoder::from_run(&serde_json::json!({
        "model_identity":{"encoder_version":7,"model_version":5,"architecture":"baylee_ai_v5_relational_v2"}
    })).unwrap();
    let mismatch = stream_game4_with_encoder_and_producer(
        b"",
        1,
        true,
        Some(&[]),
        None,
        None,
        false,
        8,
        Some(&producer7),
        |_| {
            emitted += 1;
            Ok(())
        },
    );
    assert!(
        mismatch
            .unwrap_err()
            .contains("Producer/converter encoder mismatch")
    );
    assert_eq!(emitted, 0);
    assert!(ProducerEncoder::from_run(&serde_json::json!({})).is_err());
    assert!(ProducerEncoder::from_run(&serde_json::json!({
        "model_identity":{"encoder_version":8,"model_version":5,"architecture":"baylee_ai_v5_relational_v1"}
    })).is_err());
    assert!(ProducerEncoder::from_run(&serde_json::json!({
        "model_identity":{"encoder_version":8,"model_version":5,"architecture":"baylee_ai_v5_relational_v2"}
    })).is_ok());
}

#[test]
fn candidate_context_producers_keep_their_explicit_encoder_contract() {
    for architecture in [
        "baylee_ai_v5_relational_v2_candidate_v2",
        "baylee_ai_v5_relational_v2_candidate_v3",
    ] {
        for encoder in [7, 8] {
            assert!(
                ProducerEncoder::from_run(&serde_json::json!({
                    "model_identity":{"encoder_version":encoder,"model_version":5,
                        "architecture":architecture}
                }))
                .is_ok()
            );
        }
    }
    for (encoder, architecture) in [
        (8, "baylee_ai_v5_relational_v2_candidate_v9"),
        (9, "baylee_ai_v5_relational_v2_candidate_v3"),
    ] {
        assert!(
            ProducerEncoder::from_run(&serde_json::json!({
                "model_identity":{"encoder_version":encoder,"model_version":5,
                    "architecture":architecture,"global_feat_width":82}
            }))
            .is_err()
        );
    }
}

/// The record as another build would have written it: every engine hash
/// differs, the build-stable `seen` digests stay.
fn as_another_build(record: &[u8], corrupt_seen_at: Option<u64>) -> Vec<u8> {
    let mut out = Vec::new();
    for line in record.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
        let mut value: serde_json::Value = serde_json::from_slice(line).unwrap();
        if value.get("hash").is_some() {
            value["hash"] = "0123456789abcdef".into();
        }
        if corrupt_seen_at.is_some()
            && value["n"].as_u64() == corrupt_seen_at
            && value["kind"] == "input"
        {
            value["seen"] = "fedcba9876543210".into();
        }
        out.extend(serde_json::to_vec(&value).unwrap());
        out.push(b'\n');
    }
    out
}

#[test]
fn records_from_another_build_convert_only_by_their_public_table_and_only_when_asked() {
    let a = HouseDeck::named("allytifact").unwrap();
    let b = HouseDeck::named("victory").unwrap();
    let preset = table(23, &a, &b, [AIProfile::SHARP, AIProfile::EXPERT]);
    let played = play_with_options(
        &preset,
        "t-convert4-cross-build",
        Caps {
            answers: 10_000,
            wall: Duration::from_secs(60),
        },
        true,
    );
    assert!(played.outcome.finished(), "{:?}", played.outcome);
    let (same, check) = terminal_targets(&played.record, true).unwrap();
    assert_eq!(check, ReplayCheck::Exact);

    let moved = as_another_build(&played.record, None);
    let strict = terminal_targets(&moved, false).unwrap_err();
    assert!(strict.contains("Diverged"), "{strict}");
    let (across, check) = terminal_targets(&moved, true).unwrap();
    assert_eq!(check, ReplayCheck::SeenAcrossBuilds);
    assert_eq!(across, same);

    let tampered = as_another_build(&played.record, Some(12));
    let error = terminal_targets(&tampered, true).unwrap_err();
    assert!(error.contains("after input 12"), "{error}");
}
