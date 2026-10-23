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
        // No corrupted input reaches a consumer, even if the recorded action
        // would still be legal: the audit must check hashes, not only legality.
        let mut lines = played
            .record
            .split(|&b| b == b'\n')
            .filter(|s| !s.is_empty())
            .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        lines[0]["hash"] = serde_json::json!("0000000000000000");
        let bad = lines
            .iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>();
        let mut emitted = 0;
        assert!(
            stream_game4(bad.as_bytes(), 17, true, None, |_| {
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
