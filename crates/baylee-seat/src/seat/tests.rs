//! The seat's rules, one frame at a time: what it sends, whom it asks, and
//! how it tells a question sent again from a new one.

use super::*;
use baylee_client_core::test_support::{ViewBuilder, statics};
use baylee_core::ids::PlayerId;
use baylee_core::mana::ManaColor;
use baylee_engine::choice::LegalActions;
use baylee_engine::win::{EndReason, Victor};
use baylee_view::SeatIdentity;
use prost::Message as _;

const ME: PlayerId = PlayerId::new(0);

fn frame(msg: v1::envelope::Msg) -> Vec<u8> {
    Envelope { msg: Some(msg) }.encode_to_vec()
}

/// The table: two chairs, this one called `name`, `secs` a question.
fn table(name: &str, secs: Option<u32>) -> Vec<u8> {
    let mut game = statics(0);
    game.decision_secs = secs;
    game.seats = vec![
        SeatIdentity {
            player: ME,
            display_name: name.into(),
            is_ai: false,
            away: false,
            team: None,
        },
        SeatIdentity {
            player: PlayerId::new(1),
            display_name: "Alice".into(),
            is_ai: false,
            away: false,
            team: None,
        },
    ];
    frame(v1::envelope::Msg::GameStatic(v1::GameStaticMsg {
        game_id: game.game_id.clone(),
        view_version: game.view_version,
        static_json: serde_json::to_vec(&game).unwrap(),
    }))
}

fn view_at(seq: u64, remaining_ms: Option<u32>) -> Vec<u8> {
    let mut view = ViewBuilder::new(2).build();
    view.seq = seq;
    view.decision_remaining_ms = remaining_ms;
    frame(v1::envelope::Msg::StateDelta(v1::StateDelta {
        game_id: "test-game".into(),
        seq,
        view_json: serde_json::to_vec(&view).unwrap(),
        log_json: Vec::new(),
    }))
}

fn view(seq: u64) -> Vec<u8> {
    view_at(seq, None)
}

fn asked(seq: u64, pending: &Pending) -> Vec<u8> {
    frame(v1::envelope::Msg::ChoiceRequest(v1::ChoiceRequest {
        game_id: "test-game".into(),
        seq,
        pending_json: serde_json::to_vec(pending).unwrap(),
    }))
}

fn refused(message: &str) -> Vec<u8> {
    frame(v1::envelope::Msg::Error(v1::Error {
        code: 400,
        message: message.into(),
    }))
}

/// A question the orders never answer: the mind is woken for it.
fn colour() -> Pending {
    Pending::ChooseColor {
        player: ME,
        options: vec![ManaColor::Red, ManaColor::Green],
    }
}

/// A priority round with nothing to do: the orders answer it.
fn nothing() -> Pending {
    Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            ..LegalActions::default()
        }),
    }
}

const GREEN: PlayerAction = PlayerAction::ChooseColor(ManaColor::Green);

/// A seat that has its table and said it is ready.
fn seated(config: BridgeConfig) -> SeatCore {
    let mut core = SeatCore::new(config, DeckList::default(), Disclosure::Llm);
    assert!(
        core.hear(&table("LLM-seat", None)).is_empty(),
        "no view yet"
    );
    let steps = core.hear(&view(1));
    assert!(is_ready(&steps), "{steps:?}");
    core
}

fn is_ready(steps: &[Step]) -> bool {
    matches!(
        steps,
        [Step::Send(Envelope {
            msg: Some(v1::envelope::Msg::SeatReady(_))
        })]
    )
}

/// The request the mind was handed, when the steps hand it one.
fn request(steps: &[Step]) -> Option<&Request> {
    steps.iter().find_map(|step| match step {
        Step::Ask(request) => Some(&**request),
        _ => None,
    })
}

/// The answer sent, and by whom.
fn answer(steps: &[Step]) -> Option<(&PlayerAction, By, bool)> {
    steps.iter().find_map(|step| match step {
        Step::Answer {
            action, by, paced, ..
        } => Some((action, *by, *paced)),
        _ => None,
    })
}

/// Asks the seat `pending` at `seq` and returns the question number the
/// mind was handed.
fn ask_mind(core: &mut SeatCore, seq: u64, pending: &Pending) -> u64 {
    core.hear(&view(seq));
    let steps = core.hear(&asked(seq, pending));
    request(&steps)
        .unwrap_or_else(|| panic!("the mind was not asked: {steps:?}"))
        .question
}

#[test]
fn a_chair_not_called_as_its_mind_must_be_is_left() {
    for (name, disclosure) in [
        ("Alice", Disclosure::Llm),
        ("NET-seat", Disclosure::Llm),
        ("LLM-seat", Disclosure::Net),
    ] {
        let mut core = SeatCore::new(BridgeConfig::default(), DeckList::default(), disclosure);
        core.hear(&table(name, None));
        let steps = core.hear(&view(1));
        assert!(
            matches!(steps.as_slice(), [Step::Leave(_)]),
            "{name} as {disclosure:?}: {steps:?}"
        );
        assert!(
            core.hear(&asked(2, &colour())).is_empty(),
            "a seat that left answers nothing"
        );
    }
}

#[test]
fn a_blitz_table_is_left_unless_the_bridge_is_told_to_sit() {
    let mut core = SeatCore::new(
        BridgeConfig::default(),
        DeckList::default(),
        Disclosure::Llm,
    );
    core.hear(&table("LLM-seat", Some(BLITZ_SECS)));
    assert!(matches!(core.hear(&view(1)).as_slice(), [Step::Leave(_)]));

    let config = BridgeConfig {
        allow_blitz: true,
        ..BridgeConfig::default()
    };
    let mut core = SeatCore::new(config, DeckList::default(), Disclosure::Llm);
    core.hear(&table("LLM-seat", Some(BLITZ_SECS)));
    assert!(is_ready(&core.hear(&view(1))));
    // One second more is a table a mind sits at.
    let mut core = SeatCore::new(
        BridgeConfig::default(),
        DeckList::default(),
        Disclosure::Llm,
    );
    core.hear(&table("LLM-seat", Some(BLITZ_SECS + 1)));
    assert!(is_ready(&core.hear(&view(1))));
}

#[test]
fn ready_is_said_once_a_socket() {
    let mut core = seated(BridgeConfig::default());
    assert!(core.hear(&view(2)).is_empty());
    let steps = core.resumed();
    assert!(
        matches!(
            steps.as_slice(),
            [Step::Send(Envelope {
                msg: Some(v1::envelope::Msg::Resume(v1::ResumeGame {
                    last_seq: 2,
                    ..
                }))
            })]
        ),
        "{steps:?}"
    );
    assert!(is_ready(&core.hear(&view(2))));
}

#[test]
fn the_orders_answer_what_is_no_decision_and_the_mind_the_rest() {
    let mut core = seated(BridgeConfig::default());
    let steps = core.hear(&asked(1, &nothing()));
    assert_eq!(
        answer(&steps),
        Some((&PlayerAction::PassPriority, By::Standing, false)),
        "a standing answer is not paced"
    );
    let question = ask_mind(&mut core, 2, &colour());
    let steps = core.answered(question, Ok(Answer::new(GREEN)));
    assert_eq!(answer(&steps), Some((&GREEN, By::Mind, true)));
    assert_eq!(core.stats().wakes, 1);
    assert_eq!(core.stats().questions, 2);
}

/// Every `AiLog` the steps send: its note and its reasoning.
fn ai_logs(steps: &[Step]) -> Vec<(String, String)> {
    steps
        .iter()
        .filter_map(|step| match step {
            Step::Send(Envelope {
                msg: Some(v1::envelope::Msg::AiLog(said)),
            }) => Some((said.note.clone(), said.thinking.clone())),
            _ => None,
        })
        .collect()
}

/// Three answers to the same question, each the next time it is asked: the
/// model's (a note and a reasoning, after some thought), a plan's next tap
/// (the mind's answer, with a note but no model time) and the house's. The
/// steps of each.
fn the_model_a_plan_and_the_house_answer(core: &mut SeatCore) -> [Vec<Step>; 3] {
    let question = ask_mind(core, 2, &colour());
    let thought = Answer {
        model_time: Duration::from_millis(1_200),
        note: Some(r#"{"chose":"Green"}"#.into()),
        thinking: Some("my hand is two Forests".into()),
        ..Answer::new(GREEN)
    };
    let model = core.answered(question, Ok(thought));
    let question = ask_mind(core, 3, &colour());
    let tap = Answer {
        note: Some(r#"{"plan":"cast Giant Growth"}"#.into()),
        ..Answer::new(GREEN)
    };
    let plan = core.answered(question, Ok(tap));
    let question = ask_mind(core, 4, &colour());
    let house = core.answered(question, Err(MindError::Declined("no".into())));
    [model, plan, house]
}

/// In a debug build the bridge sends what its model said beside each answer
/// the model thought about, before the answer itself, at a table with no
/// teams at all: the AI log is open to the table (`docs/protocol.md` §"An
/// AI seat's reasoning"). A plan's tap and the house's answer say nothing.
#[cfg(debug_assertions)]
#[test]
fn in_a_debug_build_the_model_s_reasoning_goes_out_beside_its_answer() {
    let mut core = seated(BridgeConfig::default());
    let [model, plan, house] = the_model_a_plan_and_the_house_answer(&mut core);
    assert_eq!(
        ai_logs(&model),
        [(
            r#"{"chose":"Green"}"#.to_string(),
            "my hand is two Forests".to_string()
        )]
    );
    assert!(
        matches!(model.last(), Some(Step::Answer { .. })),
        "the answer goes after it: {model:?}"
    );
    assert_eq!(ai_logs(&plan), [], "a plan's tap says nothing new");
    assert_eq!(ai_logs(&house), [], "the house says nothing");
}

/// In a release build the bridge never sends a reasoning.
#[cfg(not(debug_assertions))]
#[test]
fn in_a_release_build_the_model_s_reasoning_stays_with_the_bridge() {
    let mut core = seated(BridgeConfig::default());
    let [model, plan, house] = the_model_a_plan_and_the_house_answer(&mut core);
    for steps in [&model, &plan, &house] {
        assert_eq!(ai_logs(steps), [], "{steps:?}");
    }
    assert!(matches!(model.last(), Some(Step::Answer { .. })));
}

/// The AI log's texts are cut at a character boundary, never inside one.
#[test]
fn an_ai_log_text_is_cut_at_a_character_boundary() {
    assert_eq!(cut("short", 16), "short");
    // "ä" is two bytes: four of them are eight, and seven cuts the last.
    assert_eq!(cut("ääää", 7), "äää");
    assert_eq!(cut("ääää", 8), "ääää");
    let long = "€".repeat(AI_LOG_BYTES);
    let kept = cut(&long, AI_LOG_BYTES);
    assert!(kept.len() <= AI_LOG_BYTES && kept.len() > AI_LOG_BYTES - 3);
    assert!(kept.chars().all(|c| c == '€'));
}

#[test]
fn the_budget_is_the_clock_less_the_margin_and_no_time_is_the_houses() {
    let config = BridgeConfig {
        think: Duration::from_secs(5),
        margin: Duration::from_secs(3),
        ..BridgeConfig::default()
    };
    let mut core = seated(config);
    let budget = |core: &mut SeatCore, seq, remaining| {
        core.hear(&view_at(seq, remaining));
        let steps = core.hear(&asked(seq, &colour()));
        let asked = request(&steps).map(|r| (r.question, r.budget));
        let by = answer(&steps).map(|(_, by, _)| by);
        if let Some((question, _)) = asked {
            core.answered(question, Ok(Answer::new(GREEN)));
        }
        (asked.map(|(_, budget)| budget), by)
    };
    assert_eq!(
        budget(&mut core, 2, None),
        (Some(Duration::from_secs(5)), None),
        "untimed: the bridge's own bound"
    );
    assert_eq!(
        budget(&mut core, 3, Some(7_000)),
        (Some(Duration::from_secs(4)), None)
    );
    assert_eq!(
        budget(&mut core, 4, Some(60_000)),
        (Some(Duration::from_secs(5)), None)
    );
    assert_eq!(budget(&mut core, 5, Some(2_000)), (None, Some(By::House)));
    assert_eq!(core.stats().fallbacks.no_time, 1);
}

#[test]
fn a_question_sent_again_while_the_mind_thinks_is_the_same_question() {
    let mut core = seated(BridgeConfig::default());
    let question = ask_mind(&mut core, 2, &colour());
    // Another seat's setting moved `seq` under it.
    assert!(core.hear(&view(3)).is_empty());
    assert!(core.hear(&asked(3, &colour())).is_empty());
    assert_eq!(core.stats().questions, 1);
    assert_eq!(core.thinking(), Some(question));
    assert!(answer(&core.answered(question, Ok(Answer::new(GREEN)))).is_some());
}

#[test]
fn the_same_frame_twice_after_an_answer_is_answered_once() {
    let mut core = seated(BridgeConfig::default());
    let question = ask_mind(&mut core, 2, &colour());
    core.answered(question, Ok(Answer::new(GREEN)));
    assert!(core.hear(&view(2)).is_empty());
    assert!(core.hear(&asked(2, &colour())).is_empty());
    assert_eq!(core.stats().questions, 1);
}

#[test]
fn the_same_bytes_at_a_new_seq_are_a_new_decision() {
    let mut core = seated(BridgeConfig::default());
    let first = ask_mind(&mut core, 2, &colour());
    core.answered(first, Ok(Answer::new(GREEN)));
    let second = ask_mind(&mut core, 3, &colour());
    assert_ne!(first, second);
    assert_eq!(core.stats().questions, 2);
}

#[test]
fn an_answer_the_table_refuses_is_retried_told_why_and_then_the_houses() {
    let mut core = seated(BridgeConfig::default());
    let question = ask_mind(&mut core, 2, &colour());
    core.answered(question, Ok(Answer::new(GREEN)));
    // The table says no and asks again at the same `seq`.
    assert!(core.hear(&refused("not now")).is_empty());
    core.hear(&view(2));
    let steps = core.hear(&asked(2, &colour()));
    let retry = request(&steps)
        .and_then(|r| r.retry.clone())
        .expect("asked again, told why");
    assert_eq!(
        (retry.by, retry.reason.as_str()),
        (RefusedBy::Table, "not now")
    );
    let steps = core.answered(question, Ok(Answer::new(GREEN)));
    assert!(answer(&steps).is_some());
    core.hear(&refused("still not"));
    core.hear(&view(2));
    let steps = core.hear(&asked(2, &colour()));
    assert!(request(&steps).is_none(), "one retry only");
    assert_eq!(answer(&steps).map(|(_, by, _)| by), Some(By::House));
    assert_eq!(core.stats().refused_by_table, 2);
    assert_eq!(core.stats().fallbacks.refused, 1);
}

#[test]
fn a_foul_is_retried_told_why_and_then_the_houses() {
    let mut core = seated(BridgeConfig::default());
    let question = ask_mind(&mut core, 2, &colour());
    let blue = PlayerAction::ChooseColor(ManaColor::Blue);
    let steps = core.answered(question, Ok(Answer::new(blue.clone())));
    let retry = request(&steps)
        .and_then(|r| r.retry.clone())
        .expect("asked again");
    assert_eq!(retry.by, RefusedBy::Referee);
    let steps = core.answered(question, Ok(Answer::new(blue)));
    assert_eq!(answer(&steps).map(|(_, by, _)| by), Some(By::House));
    assert_eq!(core.stats().refused_by_referee, 2);
}

#[test]
fn after_a_new_socket_the_answer_that_went_down_goes_again() {
    let mut core = seated(BridgeConfig::default());
    let question = ask_mind(&mut core, 2, &colour());
    core.answered(question, Ok(Answer::new(GREEN)));
    core.resumed();
    core.hear(&view(2));
    let steps = core.hear(&asked(2, &colour()));
    assert_eq!(
        answer(&steps),
        Some((&GREEN, By::Mind, false)),
        "the same answer, unpaced"
    );
    assert!(request(&steps).is_none(), "nobody is asked twice");
    assert!(
        core.take_notes()
            .iter()
            .any(|n| matches!(n.event, Event::Resent))
    );
    // And only once.
    assert!(core.hear(&asked(2, &colour())).is_empty());
}

#[test]
fn an_answer_still_being_thought_about_at_a_new_socket_goes_once() {
    let mut core = seated(BridgeConfig::default());
    let question = ask_mind(&mut core, 2, &colour());
    core.resumed();
    core.hear(&view(2));
    assert!(
        core.hear(&asked(2, &colour())).is_empty(),
        "the mind is still on it"
    );
    assert!(answer(&core.answered(question, Ok(Answer::new(GREEN)))).is_some());
    assert!(
        core.hear(&asked(2, &colour())).is_empty(),
        "sent on the new socket, not again"
    );
}

#[test]
fn an_answer_to_a_question_that_moved_on_is_late() {
    let mut core = seated(BridgeConfig::default());
    let first = ask_mind(&mut core, 2, &colour());
    core.hear(&view(3));
    let steps = core.hear(&asked(3, &nothing()));
    assert!(
        matches!(steps.first(), Some(Step::Cancel(q)) if *q == first),
        "{steps:?}"
    );
    assert!(core.answered(first, Ok(Answer::new(GREEN))).is_empty());
    assert_eq!(core.stats().late, 1);
    assert!(core.expired(first).is_empty(), "nor does its clock run");
}

#[test]
fn a_mind_that_keeps_failing_is_taken_down_until_a_new_socket() {
    let mut core = seated(BridgeConfig::default());
    for seq in 2..5 {
        let question = ask_mind(&mut core, seq, &colour());
        let steps = core.expired(question);
        assert_eq!(answer(&steps).map(|(_, by, _)| by), Some(By::House));
        assert_eq!(
            steps.iter().any(|s| matches!(s, Step::MindDown)),
            seq == 4,
            "{seq}"
        );
    }
    core.hear(&view(5));
    let steps = core.hear(&asked(5, &colour()));
    assert!(
        request(&steps).is_none(),
        "a mind that is down is not asked"
    );
    assert_eq!(core.stats().fallbacks.down, 1);
    core.resumed();
    assert!(is_ready(&core.hear(&view(6))));
    ask_mind(&mut core, 6, &colour());
    assert_eq!(core.stats().mind_down, 1);
}

#[test]
fn an_answer_resets_the_run_of_failures() {
    let mut core = seated(BridgeConfig::default());
    for seq in 2..8 {
        let question = ask_mind(&mut core, seq, &colour());
        let steps = if seq % 3 == 1 {
            core.answered(question, Ok(Answer::new(GREEN)))
        } else {
            core.expired(question)
        };
        assert!(!steps.iter().any(|s| matches!(s, Step::MindDown)), "{seq}");
    }
}

#[test]
fn the_end_of_the_game_stops_the_thinking_and_says_who_won() {
    let mut core = seated(BridgeConfig::default());
    let question = ask_mind(&mut core, 2, &colour());
    let over = Pending::GameOver(GameResult {
        winner: Some(Victor::Player(ME)),
        reason: EndReason::LastPlayerStanding,
    });
    let steps = core.hear(&asked(3, &over));
    assert!(
        matches!(steps.first(), Some(Step::Cancel(q)) if *q == question),
        "{steps:?}"
    );
    assert!(matches!(steps.last(), Some(Step::Over(_))));
    assert!(core.is_over());
    assert_eq!(core.stats().outcome, Some(Outcome::Won));
    assert!(core.hear(&asked(4, &colour())).is_empty());
}

/// A question that offers nothing to choose from has no answer at all. It is
/// not handed to the mind or the house, nor left to a clock an untimed table
/// does not have: the seat leaves at once, naming the question, and hears
/// nothing more.
#[test]
fn a_question_that_offers_nothing_is_left_at_once_saying_which() {
    for pending in crate::scripted::tests::nothing_to_choose_from() {
        let kind = crate::scripted::kind(&pending);
        assert!(referee::offers_nothing(&pending), "{kind}");
        let mut core = seated(BridgeConfig::default());
        core.hear(&view(2));
        let steps = core.hear(&asked(2, &pending));
        let Some(Step::Leave(reason)) = steps.last() else {
            panic!("{kind}: the seat stayed: {steps:?}");
        };
        assert!(reason.contains(kind), "{kind}: {reason}");
        assert!(
            request(&steps).is_none() && answer(&steps).is_none(),
            "{kind}: {steps:?}"
        );
        assert_eq!(core.stats().unanswerable, 1, "{kind}");
        let notes = core.take_notes();
        assert!(
            notes.iter().any(|n| matches!(n.event, Event::Unanswerable)),
            "{kind}: {notes:?}"
        );
        assert!(
            matches!(notes.last().map(|n| &n.event), Some(Event::Left { .. })),
            "{kind}: {notes:?}"
        );
        assert!(core.hear(&asked(3, &colour())).is_empty(), "{kind}");
    }
    // An ordinary question offers something.
    assert!(!referee::offers_nothing(&colour()));
}

/// When the table refuses the mind's answer twice, the house's and then the
/// least answer, nothing is left to send: the seat leaves, saying so.
#[test]
fn a_seat_whose_every_answer_is_refused_leaves_saying_which() {
    let mut core = seated(BridgeConfig::default());
    let question = ask_mind(&mut core, 2, &colour());
    core.answered(question, Ok(Answer::new(GREEN)));
    let mut authors = Vec::new();
    for _ in 0..4 {
        core.hear(&refused("no"));
        core.hear(&view(2));
        let steps = core.hear(&asked(2, &colour()));
        if request(&steps).is_some() {
            authors.push(By::Mind);
            core.answered(question, Ok(Answer::new(GREEN)));
        } else if let Some((_, by, _)) = answer(&steps) {
            authors.push(by);
        } else {
            let Some(Step::Leave(reason)) = steps.last() else {
                panic!("neither an answer nor a leave: {steps:?}");
            };
            assert!(reason.contains("ChooseColor"), "{reason}");
            break;
        }
    }
    assert_eq!(authors, [By::Mind, By::House, By::Least]);
    assert_eq!(core.stats().unanswerable, 1);
    assert!(core.hear(&asked(3, &colour())).is_empty());
}

#[test]
fn a_room_that_closed_stops_the_thinking_and_says_no_result() {
    let mut core = seated(BridgeConfig::default());
    let question = ask_mind(&mut core, 2, &colour());
    core.take_notes();
    let steps = core.closed();
    assert!(
        matches!(steps.as_slice(), [Step::Cancel(q)] if *q == question),
        "{steps:?}"
    );
    assert!(core.is_over());
    assert_eq!(core.stats().outcome, None, "no result was seen");
    let notes = core.take_notes();
    assert!(
        matches!(notes.as_slice(), [note] if matches!(note.event, Event::Closed)),
        "{notes:?}"
    );
    // Nothing is asked or answered after it, and it is said once.
    assert!(core.answered(question, Ok(Answer::new(GREEN))).is_empty());
    assert!(core.hear(&asked(3, &colour())).is_empty());
    assert!(core.closed().is_empty());
}

#[test]
fn every_answer_names_the_game_and_no_token() {
    let mut core = seated(BridgeConfig::default());
    let steps = core.hear(&asked(1, &nothing()));
    let Some(Step::Answer { envelope, .. }) = steps.first() else {
        panic!("{steps:?}");
    };
    let Some(v1::envelope::Msg::PlayerAction(msg)) = &envelope.msg else {
        panic!("{envelope:?}");
    };
    assert_eq!(msg.game_id, "test-game");
    assert!(msg.seat_token.is_empty());
    assert_eq!(
        serde_json::from_slice::<PlayerAction>(&msg.action_json).unwrap(),
        PlayerAction::PassPriority
    );
}

/// What the steps send, by kind: `"mind:<model>"` for a declaration,
/// `"ready"`, `"resume"`, and `"other"`.
fn sent(steps: &[Step]) -> Vec<String> {
    steps
        .iter()
        .filter_map(|step| match step {
            Step::Send(Envelope { msg: Some(msg) }) => Some(match msg {
                v1::envelope::Msg::SeatMind(mind) => format!("mind:{}", mind.model),
                v1::envelope::Msg::SeatReady(_) => "ready".into(),
                v1::envelope::Msg::Resume(_) => "resume".into(),
                _ => "other".into(),
            }),
            _ => None,
        })
        .collect()
}

/// The seat says what answers it on every socket, before it says it is
/// ready and so before its first answer; a swap is told at once, and
/// declaring the same again sends nothing (`docs/protocol.md` §"Who
/// answers a seat, as it says").
#[test]
fn the_seat_declares_its_mind_before_its_ready_on_every_socket_and_on_a_swap() {
    let model = |id: &str| v1::SeatMind {
        kind: v1::seat_mind::Kind::LlmApi as i32,
        provider: "anthropic".into(),
        model: id.into(),
        effort: "high".into(),
        level: String::new(),
    };
    let mut core = SeatCore::new(
        BridgeConfig::default(),
        DeckList::default(),
        Disclosure::Llm,
    )
    .with_mind(model("claude-opus-5-5"));
    assert!(
        core.hear(&table("LLM-seat", None)).is_empty(),
        "no view yet"
    );
    assert_eq!(
        sent(&core.hear(&view(1))),
        ["mind:claude-opus-5-5", "ready"]
    );
    assert_eq!(
        sent(&core.hear(&view(2))),
        Vec::<String>::new(),
        "once a socket"
    );
    assert!(
        core.declare(model("claude-opus-5-5")).is_empty(),
        "the same"
    );
    assert_eq!(
        sent(&core.declare(model("claude-sonnet-5-5"))),
        ["mind:claude-sonnet-5-5"]
    );
    assert_eq!(sent(&core.resumed()), ["resume"]);
    assert_eq!(
        sent(&core.hear(&table("LLM-seat", None))),
        ["mind:claude-sonnet-5-5", "ready"],
        "the new socket is told the mind that answers now"
    );
    for declared in [model("claude-opus-5-5"), model("claude-sonnet-5-5")] {
        assert_eq!(baylee_protocol::mind::fault(&declared), None);
    }
}

/// A hosted seat names nobody to its mind: the request's context holds no
/// display name, and the prompt's prefix says `P2 is an opponent`.
#[test]
fn a_pseudonymous_seat_tells_its_mind_no_name() {
    for pseudonymous in [false, true] {
        let mut core = seated(BridgeConfig {
            pseudonymous,
            ..BridgeConfig::default()
        });
        let steps = core.hear(&asked(2, &colour()));
        let request = request(&steps).expect("the mind is asked");
        let prefix = crate::narrator::prefix(&request.context, crate::narrator::DeckText::Full);
        assert_eq!(prefix.contains("Alice"), !pseudonymous, "{prefix}");
        assert_eq!(
            request.context.names.iter().any(|n| !n.is_empty()),
            !pseudonymous
        );
        if pseudonymous {
            assert!(prefix.contains("P2 is an opponent."), "{prefix}");
        }
    }
}
