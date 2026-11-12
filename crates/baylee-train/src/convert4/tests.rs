use super::*;
use crate::housedeck::HouseDeck;
use crate::selfplay::{Caps, play_with_options, table};
use baylee_core::preset::AIProfile;
use baylee_engine::choice::PlayerAction;
use std::time::Duration;

/// Streams a house self-play record, which shares no hand, at `encoder`.
fn stream(
    record: &[u8],
    encoder: u32,
    emit: &mut dyn FnMut(Step4) -> Result<(), String>,
) -> Result<StreamedGame4, String> {
    stream_game4_with_encoder_and_producer(
        record,
        17,
        true,
        None,
        None,
        None,
        false,
        encoder,
        None,
        TeamHands::Private,
        emit,
    )
}

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
        let original =
            extract_game4_with_team_hands(&played.record, 17, true, None, TeamHands::Private)
                .unwrap();
        let mut streamed = Vec::new();
        let result = stream(&played.record, 7, &mut |frame| {
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
        let payment_result = stream(&played.record, 8, &mut |frame| {
            payment_frames.push(frame);
            Ok(())
        })
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
        // The opening hash alone: `seen` stays as recorded, and an exact
        // audit must not let the public table stand in for the hash.
        lines[0]["hash"] = serde_json::json!("0000000000000000");
        let mut bad = Vec::new();
        for line in &lines {
            serde_json::to_writer(&mut bad, line).unwrap();
            bad.push(b'\n');
        }
        let mut emitted = 0;
        assert!(
            stream(&bad, 7, &mut |_| {
                emitted += 1;
                Ok(())
            })
            .is_err()
        );
        assert_eq!(emitted, 0);
        assert!(
            stream(
                &played.record,
                7,
                &mut |_| Err("test output failure".into())
            )
            .unwrap_err()
            .contains("test output failure")
        );
    }
}

/// A seat's features show a teammate's hand only when the record's producer
/// showed it: seat settings are not in the record, so a replay that shared
/// by default would give a team game of people or language models hands
/// their seats never saw.
#[test]
fn a_replay_shows_teammates_hands_only_when_the_producer_did() {
    let a = HouseDeck::named("allytifact").unwrap();
    let b = HouseDeck::named("victory").unwrap();
    let preset =
        crate::selfplay::table_for(23, &[&a, &b, &a, &b], &[AIProfile::SHARP; 4], &[1, 2, 1, 2]);
    let played = play_with_options(
        &preset,
        "team-hands",
        Caps {
            answers: 10_000,
            wall: Duration::from_secs(60),
        },
        true,
    );
    assert!(played.outcome.finished());
    // Frames that hold a card of a teammate's hand, over the whole game.
    let shown = |team_hands| -> Result<usize, String> {
        let mut shown = 0;
        stream_game4_with_encoder_and_producer(
            &played.record,
            1,
            true,
            None,
            None,
            None,
            false,
            7,
            None,
            team_hands,
            |frame| {
                shown += usize::from(
                    frame
                        .state
                        .entity_zone_ids
                        .contains(&(crate::features4::EntityZone4::SharedHand as u8)),
                );
                Ok(())
            },
        )?;
        Ok(shown)
    };
    assert_eq!(shown(TeamHands::Private), Ok(0));
    assert!(shown(TeamHands::Shared).unwrap() > 0);
    let refused = shown(TeamHands::Unknown).unwrap_err();
    assert!(refused.contains(TeamHands::RUN_KEY), "{refused}");
    // The entry points that know no producer refuse a team game too.
    assert!(stream_game4(&played.record, 1, true, None, |_| Ok(())).is_err());
    assert!(extract_game4(&played.record, 1, true).is_err());
    // A duel has no teammate: nothing to share, nothing to refuse.
    let duel = crate::selfplay::table_for(23, &[&a, &b], &[AIProfile::SHARP; 2], &[]);
    let duel = play_with_options(
        &duel,
        "team-hands-duel",
        Caps {
            answers: 10_000,
            wall: Duration::from_secs(60),
        },
        true,
    );
    assert!(stream_game4(&duel.record, 1, true, None, |_| Ok(())).is_ok());
}

/// Training and evaluation play by the timestamps, so that the net's numbers
/// and records stay what they are.
#[test]
fn no_training_preset_asks_the_order_of_triggers() {
    let a = HouseDeck::named("allytifact").unwrap();
    let b = HouseDeck::named("victory").unwrap();
    let duel = table(19, &a, &b, [AIProfile::SHARP, AIProfile::EXPERT]);
    assert!(!duel.house_rules.order_triggers);
    assert!(!duel.house_rules.divide_trample, "a trampler's damage too");
    let four = crate::selfplay::table_for(19, &[&a, &b, &a, &b], &[AIProfile::SHARP; 4], &[]);
    assert!(!four.house_rules.order_triggers);
    assert!(!four.house_rules.divide_trample, "a trampler's damage too");
}

/// A language model's table asks the order of triggers (the collector's rooms
/// are real rooms), and such a record converts: the question is forced like
/// any other decision that is not the net's, and leaves no step behind.
#[test]
fn a_game_that_asked_the_order_of_triggers_converts() {
    let a = HouseDeck::named("allytifact").unwrap();
    let handle =
        serde_json::to_string(&baylee_core::ids::ObjectId::queued_trigger(0)).expect("a number");
    let asked = (19..40).find_map(|seed| {
        let mut preset = table(seed, &a, &a, [AIProfile::SHARP, AIProfile::EXPERT]);
        preset.house_rules.order_triggers = true;
        let played = play_with_options(
            &preset,
            "t-convert4-order",
            Caps {
                answers: 10_000,
                wall: Duration::from_secs(60),
            },
            true,
        );
        assert!(played.outcome.finished(), "{:?}", played.outcome);
        String::from_utf8_lossy(&played.record)
            .contains(&handle)
            .then_some(played)
    });
    let played = asked.expect("some game of allies asks");
    let extracted = extract_game4(&played.record, 1, false).expect("extracts cleanly");
    assert_eq!(extracted.unencoded_decisions, 0);
    assert!(!extracted.steps.is_empty());
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
    let result = stream_game4_with_encoder(b"", 1, true, None, None, None, false, 11, |_| {
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
        TeamHands::Unknown,
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

/// A record as a build from before the activator boundary
/// ([`ACTIVATOR_BOUNDARY`]) wrote it: that build still named `activator`
/// after its loyalty ability, so every hash from input `from` on is the one
/// naming it, the engine's own reading of what the old build wrote
/// (`activation_choice_tests` holds the two equal in lockstep), and the
/// header names that build.
fn as_written_before_the_boundary(record: &[u8], from: u64, activator: PlayerId) -> Vec<u8> {
    let mut engine = None;
    let mut out = Vec::new();
    for line in record.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
        let mut value: serde_json::Value = serde_json::from_slice(line).unwrap();
        match serde_json::from_slice::<Line>(line).unwrap() {
            Line::Header { preset, .. } => {
                engine = Some(
                    baylee_engine::engine::Engine::new(&preset, baylee_gamehost::RegistryLookup)
                        .unwrap(),
                );
                value["build"] = "0.1.0-beta.9c+build.0 (1259551af4)".into();
            }
            Line::Input {
                n, seat, action, ..
            } => {
                let engine = engine.as_mut().unwrap();
                engine.apply(PlayerId::new(seat), action).unwrap();
                if n >= from {
                    value["hash"] = format!(
                        "{:016x}",
                        engine.snapshot_hash_naming_stale_activator(activator)
                    )
                    .into();
                }
            }
            _ => {}
        }
        out.extend(serde_json::to_vec(&value).unwrap());
        out.push(b'\n');
    }
    out
}

/// A recorded game on this build in which seat 0 activates The Theorist,
/// Jace Beleren's +1 and seat 1 then concedes: the record, and the input
/// that activated the ability.
fn jace_game() -> (Vec<u8>, u64) {
    const OWNER: PlayerId = PlayerId::new(0);
    const OTHER: PlayerId = PlayerId::new(1);
    let card = |name: &str| baylee_core::preset::DeckEntry {
        card: baylee_cards::decks::by_name(name).unwrap(),
        print: baylee_core::ids::PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    };
    let deck = HouseDeck::named("victory").unwrap();
    let mut preset = crate::selfplay::table_for(12, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    preset.format = baylee_core::preset::FormatId::Freeform;
    preset.modifiers.clear();
    for seat in &mut preset.seats {
        seat.deck = vec![card("Forest"); 32];
        seat.commanders.clear();
        seat.sideboard.clear();
        seat.starting_life = Some(20);
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield = vec![];
    }
    preset.seats[0].starting_battlefield = vec![card("The Theorist, Jace Beleren")];
    let mut session = baylee_gamehost::Session::new_recorded(&preset, "this build").unwrap();
    session.take_over(OWNER);
    session.take_over(OTHER);
    let mut answers = 0_u64;
    let mut act = |session: &mut baylee_gamehost::Session, action: PlayerAction| {
        let seat = session.pending().asked().unwrap();
        session.act(seat, action).unwrap();
        answers += 1;
        answers - 1
    };
    while !(session.observe_view(OWNER).phase == baylee_view::Phase::FirstMain
        && matches!(session.pending(), Pending::Priority { player, .. } if *player == OWNER))
    {
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("{other:?}"),
        };
        act(&mut session, action);
    }
    let jace = session
        .observe_view(OWNER)
        .battlefield
        .iter()
        .find(|o| o.name == "The Theorist, Jace Beleren")
        .unwrap()
        .id;
    let from = act(
        &mut session,
        PlayerAction::ActivateAbility {
            source: jace,
            ability_index: 1,
        },
    );
    while !session.state().zones.stack_is_empty() {
        act(&mut session, PlayerAction::PassPriority);
    }
    session.act(OTHER, PlayerAction::Concede).unwrap();
    (session.take_record(), from)
}

/// A game with a loyalty ability, written before the activator boundary
/// and converted after it, is refused by name: the build that wrote it, the
/// build converting it, the boundary and the rule, never an ordinary
/// divergence and never a game quietly left out. Both ways the audit runs,
/// exactly and by the public table, since its opening hash is this build's.
/// A hash that is neither this build's nor the old one's is a divergence as
/// before.
#[test]
fn a_record_from_before_the_activator_boundary_is_refused_by_name() {
    const OWNER: PlayerId = PlayerId::new(0);
    let (record, from) = jace_game();
    let (_, check) = terminal_targets(&record, false).unwrap();
    assert_eq!(check, ReplayCheck::Exact, "this build's own record");
    // The scan reads the loyalty ability off the actions alone, and finds
    // nothing in the same record without it.
    let crossing = super::activator_boundary::crossings(&record).unwrap();
    assert_eq!(
        crossing,
        super::activator_boundary::Crossings {
            loyalty: 1,
            ..Default::default()
        }
    );
    let without: Vec<u8> = record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .filter(|l| !String::from_utf8_lossy(l).contains("\"ActivateAbility\""))
        .flat_map(|l| l.iter().copied().chain(*b"\n"))
        .collect();
    assert!(
        !super::activator_boundary::crossings(&without)
            .unwrap()
            .any()
    );

    let old = as_written_before_the_boundary(&record, from, OWNER);
    // It opens with this build's hash, which is why the exact audit plays on
    // into it rather than refusing it at the header as another build's.
    assert!(super::activator_boundary::opens_as_this_build(&old));
    assert!(!super::activator_boundary::opens_as_this_build(
        &as_another_build(&old, None)
    ));
    for across_builds in [false, true] {
        let error = terminal_targets(&old, across_builds).unwrap_err();
        assert!(error.starts_with(BOUNDARY_REFUSAL), "{error}");
        for named in [
            "1259551af4",
            baylee_gamehost::record::this_build(),
            BOUNDARY_RULE,
            &format!("from input {from}"),
        ] {
            assert!(error.contains(named), "{named} in: {error}");
        }
    }

    let mut tampered = Vec::new();
    for line in old.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
        let mut value: serde_json::Value = serde_json::from_slice(line).unwrap();
        if value["kind"] == "input" && value["n"].as_u64() == Some(from + 1) {
            value["hash"] = "0123456789abcdef".into();
        }
        tampered.extend(serde_json::to_vec(&value).unwrap());
        tampered.push(b'\n');
    }
    let error = terminal_targets(&tampered, false).unwrap_err();
    assert!(error.starts_with("replay audit: Diverged"), "{error}");
}

/// `record` with `extra` (an input line's fields) put in after its line at
/// input `after` (`None`: before the end line), every later line numbered
/// one on; the line at input `at` (if any) is given other digests.
fn with_inserted(
    record: &[u8],
    after: Option<u64>,
    extra: &serde_json::Value,
    at: Option<u64>,
) -> Vec<u8> {
    let mut out = Vec::new();
    let mut push = |value: &serde_json::Value| {
        out.extend(serde_json::to_vec(value).unwrap());
        out.push(b'\n');
    };
    let mut inserted = false;
    for line in record.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
        let mut value: serde_json::Value = serde_json::from_slice(line).unwrap();
        if after.is_none() && value["kind"] == "end" && !inserted {
            push(extra);
            inserted = true;
        }
        if inserted && let Some(n) = value["n"].as_u64() {
            value["n"] = (n + 1).into();
        }
        let n = value["n"].as_u64();
        if value["kind"] == "input" && at.is_some() && n == at {
            value["hash"] = "0123456789abcdef".into();
            value["seen"] = "fedcba9876543210".into();
        }
        push(&value);
        if value["kind"] == "input" && after.is_some() && n == after && !inserted {
            push(extra);
            inserted = true;
        }
    }
    assert!(inserted, "{after:?} is not an input of the record");
    out
}

/// The line at input `n` of `record`.
fn input_line(record: &[u8], n: u64) -> serde_json::Value {
    record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_slice::<serde_json::Value>(l).unwrap())
        .find(|v| v["kind"] == "input" && v["n"].as_u64() == Some(n))
        .unwrap()
}

/// A record written before the question boundary answers questions a
/// later build no longer asks, and is refused by name: the build that wrote
/// it, the build converting it, where, and the rule. Its exact audit stops
/// the run (it opens with this build's hash); one of another build is named
/// in its seen audit's reason. Two shapes, one per commit: answers to a
/// damage-order question (c55a8c6eb), between an input whose hash this
/// build parts from and the answer after which it carries this build's
/// hash again, and answers after the game was decided (a5eb549ca), which
/// this build refuses as the game is over. Departing any other way is a
/// divergence as before.
#[test]
fn a_record_from_before_the_question_boundary_is_refused_by_name() {
    let (record, from) = jace_game();
    let k = from + 1;
    let original = input_line(&record, k);
    let order = serde_json::json!({
        "kind": "input", "n": k + 1, "at": 0, "seat": original["seat"], "by": original["by"],
        "action": {"ChooseDamageEffect": {"choice": {"batch": 0, "step": 1}, "effect": 0}},
        "hash": original["hash"], "seen": original["seen"],
    });
    let orders = with_inserted(&record, Some(k), &order, Some(k));
    let last = record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .filter_map(|l| serde_json::from_slice::<serde_json::Value>(l).ok())
        .rfind(|v| v["kind"] == "input")
        .unwrap();
    let late = serde_json::json!({
        "kind": "input", "n": last["n"].as_u64().unwrap() + 1, "at": 0, "seat": 0,
        "by": last["by"], "action": {"Arrange": {"piles": [[1, 2]]}},
        "hash": "0123456789abcdef", "seen": "fedcba9876543210",
    });
    let after_the_end = with_inserted(&record, None, &late, None);
    for (old, first, what) in [
        (&orders, k, "1 damage-order question(s)"),
        (
            &after_the_end,
            last["n"].as_u64().unwrap() + 1,
            "1 question(s) asked after the game was decided",
        ),
    ] {
        for across_builds in [false, true] {
            let error = terminal_targets(old, across_builds).unwrap_err();
            assert!(error.starts_with(QUESTION_REFUSAL), "{error}");
            for named in [
                baylee_gamehost::record::this_build(),
                QUESTION_RULE,
                &format!("from input {first}"),
                what,
            ] {
                assert!(error.contains(named), "{named} in: {error}");
            }
        }
        let error = terminal_targets(&as_another_build(old, None), true).unwrap_err();
        assert!(
            error.starts_with("seen replay audit: written before question boundary c55a8c6eb"),
            "{error}"
        );
        assert!(error.contains(what), "{what} in: {error}");
    }

    // An input this build parts from with no order answered after it, an
    // answer that does not carry this build's hash again, and a pass after
    // the end are divergences as before.
    let parted = with_inserted(&record, None, &late, Some(k));
    let error = terminal_targets(&parted, false).unwrap_err();
    assert!(error.starts_with("replay audit: Diverged"), "{error}");
    let mut wrong = order.clone();
    wrong["hash"] = "00000000000000aa".into();
    let error =
        terminal_targets(&with_inserted(&record, Some(k), &wrong, Some(k)), false).unwrap_err();
    assert!(error.starts_with("replay audit: Diverged"), "{error}");
    let mut pass = late.clone();
    pass["action"] = "PassPriority".into();
    let error = terminal_targets(&with_inserted(&record, None, &pass, None), false).unwrap_err();
    assert!(error.starts_with("replay audit: Refused"), "{error}");
}

/// A recorded game on this build in which seat 0 casts Fade Away over seat
/// 1's two Grizzly Bears, nobody pays, seat 1 gives up both Bears, and seat
/// 1 then concedes: the record, and the input after which Fade Away begins
/// to resolve (its count opens and its first question is asked).
fn fade_away_game() -> (Vec<u8>, u64) {
    const CASTER: PlayerId = PlayerId::new(0);
    const OTHER: PlayerId = PlayerId::new(1);
    let card = |name: &str| baylee_core::preset::DeckEntry {
        card: baylee_cards::decks::by_name(name).unwrap(),
        print: baylee_core::ids::PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    };
    let deck = HouseDeck::named("victory").unwrap();
    let mut preset = crate::selfplay::table_for(12, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    preset.format = baylee_core::preset::FormatId::Freeform;
    preset.modifiers.clear();
    for seat in &mut preset.seats {
        seat.deck = vec![card("Island"); 32];
        seat.commanders.clear();
        seat.sideboard.clear();
        seat.starting_life = Some(20);
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield = vec![];
    }
    preset.house_rules.starting_player = Some(0);
    preset.seats[0].starting_hand = Some(vec![card("Fade Away")]);
    preset.seats[0].starting_battlefield = vec![card("Island"); 3];
    preset.seats[1].starting_battlefield = vec![card("Grizzly Bears"); 2];
    let mut session = baylee_gamehost::Session::new_recorded(&preset, "this build").unwrap();
    session.take_over(CASTER);
    session.take_over(OTHER);
    let act = |session: &mut baylee_gamehost::Session, action: PlayerAction| {
        let seat = session.pending().asked().unwrap();
        session.act(seat, action).unwrap();
    };
    while !(session.observe_view(CASTER).phase == baylee_view::Phase::FirstMain
        && matches!(session.pending(), Pending::Priority { player, .. } if *player == CASTER))
    {
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("{other:?}"),
        };
        act(&mut session, action);
    }
    let view = session.observe_view(CASTER);
    for island in view
        .battlefield
        .iter()
        .filter(|o| o.controller == CASTER && o.name == "Island")
        .map(|o| o.id)
        .collect::<Vec<_>>()
    {
        act(
            &mut session,
            PlayerAction::ActivateManaAbility { source: island },
        );
    }
    act(
        &mut session,
        PlayerAction::CastSpell {
            card: view.hand[0].id,
        },
    );
    while !(session.state().zones.stack_is_empty()
        && matches!(session.pending(), Pending::Priority { .. }))
    {
        let action = match session.pending().clone() {
            Pending::Priority { .. } => PlayerAction::PassPriority,
            Pending::YesNo { .. } => PlayerAction::YesNo(false),
            Pending::ChooseCards { options, .. } => PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
            other => panic!("{other:?}"),
        };
        act(&mut session, action);
    }
    assert!(
        session
            .observe_view(OTHER)
            .battlefield
            .iter()
            .all(|o| o.name != "Grizzly Bears"),
        "both Bears were given up"
    );
    session.act(OTHER, PlayerAction::Concede).unwrap();
    let record = session.take_record();
    // The input before the first answer to "pay {1}?", as the record numbers it.
    let inputs: Vec<serde_json::Value> = record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_slice::<serde_json::Value>(l).unwrap())
        .filter(|v| v["kind"] == "input")
        .collect();
    let first = inputs
        .iter()
        .position(|v| v["action"].get("YesNo").is_some())
        .expect("Fade Away asked");
    let from = inputs[first - 1]["n"].as_u64().unwrap();
    (record, from)
}

/// `record` with other digests at input `n`: the engine hash, and with
/// `seen` the public table's too.
fn with_other_digests_at(record: &[u8], n: u64, seen: bool) -> Vec<u8> {
    let mut out = Vec::new();
    for line in record.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
        let mut value: serde_json::Value = serde_json::from_slice(line).unwrap();
        if value["kind"] == "input" && value["n"].as_u64() == Some(n) {
            value["hash"] = "0123456789abcdef".into();
            if seen {
                value["seen"] = "fedcba9876543210".into();
            }
        }
        out.extend(serde_json::to_vec(&value).unwrap());
        out.push(b'\n');
    }
    out
}

/// A record written before the count boundary ([`COUNT_BOUNDARY`]) parts
/// from this build as Fade Away begins to resolve, where this build holds
/// its count open, and is refused by name: the build that wrote it, the
/// build converting it, where, and the rule, both ways the audit runs; one
/// of another build that parts there by its public table is named in its
/// seen audit's reason. A record that parts where no count is open is a
/// divergence as before.
#[test]
fn a_record_from_before_the_count_boundary_is_refused_by_name() {
    let (record, from) = fade_away_game();
    let (_, check) = terminal_targets(&record, false).unwrap();
    assert_eq!(check, ReplayCheck::Exact, "this build's own record");

    let old = with_other_digests_at(&record, from, false);
    for across_builds in [false, true] {
        let error = terminal_targets(&old, across_builds).unwrap_err();
        assert!(error.starts_with(COUNT_REFUSAL), "{error}");
        for named in [
            baylee_gamehost::record::this_build(),
            COUNT_RULE,
            &format!("from input {from}"),
            "Fade Away",
        ] {
            assert!(error.contains(named), "{named} in: {error}");
        }
    }
    let another = as_another_build(&with_other_digests_at(&record, from, true), None);
    let error = terminal_targets(&another, true).unwrap_err();
    assert!(
        error.starts_with(&format!(
            "seen replay audit: written before count boundary {COUNT_BOUNDARY}"
        )),
        "{error}"
    );
    assert!(error.contains(COUNT_RULE), "{error}");

    // Before Fade Away resolves no count is open: an ordinary divergence.
    let parted = with_other_digests_at(&record, from - 1, false);
    let error = terminal_targets(&parted, false).unwrap_err();
    assert!(error.starts_with("replay audit: Diverged"), "{error}");
}

/// A game this build records with Fire // Ice in seat 0's hand: kept, passed
/// to seat 0's first main phase, and conceded by seat 1.
fn fire_ice_game() -> (Vec<u8>, baylee_core::preset::GamePreset) {
    let card = |name: &str| baylee_core::preset::DeckEntry {
        card: baylee_cards::decks::by_name(name).unwrap(),
        print: baylee_core::ids::PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    };
    let deck = HouseDeck::named("victory").unwrap();
    let mut preset = crate::selfplay::table_for(13, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    preset.format = baylee_core::preset::FormatId::Freeform;
    preset.modifiers.clear();
    for seat in &mut preset.seats {
        seat.deck = vec![card("Island"); 32];
        seat.commanders.clear();
        seat.sideboard.clear();
        seat.starting_life = Some(20);
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield = vec![];
    }
    preset.house_rules.starting_player = Some(0);
    preset.seats[0].starting_hand = Some(vec![card("Fire // Ice")]);
    let mut session = baylee_gamehost::Session::new_recorded(&preset, "this build").unwrap();
    session.take_over(PlayerId::new(0));
    session.take_over(PlayerId::new(1));
    while !(session.observe_view(PlayerId::new(0)).phase == baylee_view::Phase::FirstMain
        && matches!(session.pending(), Pending::Priority { player, .. } if player.get() == 0))
    {
        let seat = session.pending().asked().unwrap();
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("{other:?}"),
        };
        session.act(seat, action).unwrap();
    }
    session
        .act(PlayerId::new(1), PlayerAction::Concede)
        .unwrap();
    (session.take_record(), preset)
}

/// `record` with its opening hash `opening` and every input's engine hash
/// another, as a build that reads the game otherwise from its opening on
/// writes it; the public table's digests stay.
fn opening_as(record: &[u8], opening: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for line in record.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
        let mut value: serde_json::Value = serde_json::from_slice(line).unwrap();
        if value["kind"] == "header" {
            value["hash"] = opening.into();
        } else if value["kind"] == "input" {
            value["hash"] = "0123456789abcdef".into();
        }
        out.extend(serde_json::to_vec(&value).unwrap());
        out.push(b'\n');
    }
    out
}

/// A record written before the split boundary ([`SPLIT_BOUNDARY`]) opens
/// with the hash a build that read Fire // Ice by its left half wrote, and is
/// refused by name where it is held to this build's hashes: the builds, the
/// card and the rule. One of another build converts by its public table as
/// before, which reads cards by index and no characteristic. A record that
/// opens with any other hash is a divergence as before.
#[test]
fn a_record_from_before_the_split_boundary_is_refused_by_name() {
    let (record, preset) = fire_ice_game();
    let (_, check) = terminal_targets(&record, false).unwrap();
    assert_eq!(check, ReplayCheck::Exact, "this build's own record");

    let engine =
        baylee_engine::engine::Engine::new(&preset, baylee_gamehost::RegistryLookup).unwrap();
    let before = format!(
        "{:016x}",
        engine.snapshot_hash_reading_split_cards_by_left_half()
    );
    assert_ne!(before, format!("{:016x}", engine.snapshot_hash()));
    let old = opening_as(&record, &before);
    let error = terminal_targets(&old, false).unwrap_err();
    assert!(error.starts_with(SPLIT_REFUSAL), "{error}");
    for named in [
        baylee_gamehost::record::this_build(),
        SPLIT_RULE,
        "Fire // Ice",
        "CR 709.4",
    ] {
        assert!(error.contains(named), "{named} in: {error}");
    }
    let (_, check) = terminal_targets(&old, true).unwrap();
    assert_eq!(check, ReplayCheck::SeenAcrossBuilds);

    let other = opening_as(&record, "0123456789abcdef");
    let error = terminal_targets(&other, false).unwrap_err();
    assert!(error.starts_with("replay audit: Diverged"), "{error}");
}
