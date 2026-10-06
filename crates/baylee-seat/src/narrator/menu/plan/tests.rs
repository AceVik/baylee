//! The plan's grammar, and an answer that carries a plan read against its
//! question.

use super::*;
use crate::narrator::tests::{board, id, priority, request};
use crate::narrator::{Act, Narrator};
use serde_json::json;

/// Every line of the grammar reads, and writes back as it was written.
#[test]
fn every_step_of_the_grammar_reads_and_writes_back() {
    for written in [
        "play #52",
        "cast #50",
        "cast #50 -> #45 P2",
        "activate #31",
        "activate #209:2 -> P2",
        "choose Swamp",
        "choose #312, Island",
        "yes",
        "no",
        "color U",
        "mode m2",
        "n 3",
        "attack none",
        "attack #30@P2 #45@P3",
        "pass",
    ] {
        let step = Step::parse(written).unwrap_or_else(|why| panic!("{why}"));
        assert_eq!(step.to_string(), written);
        assert_eq!(Step::parse(&step.to_string()), Ok(step));
    }
    // Lenient where the meaning is plain: case, spaces, commas.
    assert_eq!(
        Step::parse("  Cast #50 ->#45, p2 "),
        Ok(Step::Cast {
            card: "#50".into(),
            targets: vec!["#45".into(), "p2".into()],
        })
    );
    assert_eq!(Step::parse("colour blue"), Ok(Step::Color("blue".into())));
    assert_eq!(
        Step::parse("attack #30@P2, #31@P3"),
        Ok(Step::Attack(vec![
            ("#30".into(), "P2".into()),
            ("#31".into(), "P3".into())
        ]))
    );
}

/// A step outside the grammar is refused with what was wrong and the
/// grammar, never read as the nearest step.
#[test]
fn a_step_outside_the_grammar_refuses_with_the_reason() {
    for (bad, why) in [
        ("tap #22", "is not a step"),
        ("cast Lightning Bolt", "object id"),
        ("attack #30", "attacker@defender"),
        ("n three", "whole number"),
        ("cast #50 -> Serra", "not an object id"),
        ("activate #31:0", "number from 1"),
        ("pass the turn", "is not a step"),
        ("choose", "names nothing"),
    ] {
        let refused = Step::parse(bad).expect_err(bad);
        assert!(refused.contains(why), "{bad}: {refused}");
        assert!(refused.contains(GRAMMAR), "{refused}");
    }
}

/// An answer's plan, `until` and `react` are read with the answer: one
/// step that is not understood refuses the whole answer, `hold` still
/// reads as `until`, and `then.targets` still names the pick's targets.
#[test]
fn an_answer_with_a_plan_is_read_whole_or_refused_whole() {
    let (view, log) = board();
    let pending = priority(&view);
    let request = request(view, pending, log);
    let menu = Narrator::new(&request.context).menu(&request);
    let answer =
        |value: serde_json::Value| menu.resolve(&Decision::from_decide(&value).expect("an object"));
    let planned = answer(json!({
        "ask": "q12", "pick": ["a1"],
        "plan": ["cast #50 -> #45", "pass"], "until": "my_turn", "react": "targets_me",
        "board": "full"
    }))
    .expect("a plan");
    assert_eq!(
        planned.act,
        Act::Now(PlayerAction::PlayLand { card: id(52) })
    );
    assert_eq!(
        planned.plan,
        [
            Step::Cast {
                card: "#50".into(),
                targets: vec!["#45".into()]
            },
            Step::Pass
        ]
    );
    assert_eq!(
        (planned.until, planned.react, planned.board_full),
        (Some(Until::MyTurn), React::TargetsMe, true)
    );
    let refused = answer(json!({"ask": "q12", "pick": ["a1"], "plan": ["cast #50", "dance"]}))
        .expect_err("a step outside the grammar");
    assert!(refused.contains("«dance»"), "{refused}");
    for (field, value) in [("until", "tomorrow"), ("react", "some"), ("board", "half")] {
        let refused = answer(json!({"ask": "q12", "pick": ["a1"], field: value})).expect_err(field);
        assert!(refused.contains(value), "{refused}");
    }
    let held = answer(json!({"ask": "q12", "pick": ["p"], "hold": "until_my_turn"})).expect("hold");
    assert_eq!(held.until, Some(Until::MyTurn));
    let none = answer(json!({"ask": "q12", "pick": ["p"], "until": "none"})).expect("none");
    assert_eq!((none.until, none.react), (None, React::All));
    let hinted =
        answer(json!({"ask": "q12", "pick": ["a2"], "then": {"targets": ["#45"]}})).expect("then");
    assert_eq!(hinted.hint.map(|h| h.objects), Some(vec![id(45)]));
}

/// The system prompt and the schema state the grammar the parser reads.
#[test]
fn the_prompt_states_the_grammar_the_parser_reads() {
    for alternative in GRAMMAR.split(" | ") {
        assert!(
            crate::llm::prompt::SYSTEM.contains(alternative),
            "the system prompt lacks «{alternative}»"
        );
    }
    let schema = crate::llm::prompt::decide_schema();
    let described = schema["properties"]["plan"]["description"]
        .as_str()
        .expect("a description");
    assert!(described.contains(GRAMMAR), "{described}");
    for until in ["my_turn", "my_main2", "my_end", "end_of_turn", "none"] {
        assert!(Until::parse(until).is_ok(), "{until}");
        assert!(
            schema["properties"]["until"]["enum"]
                .as_array()
                .expect("an enum")
                .contains(&json!(until))
        );
    }
    for react in ["all", "targets_me", "none"] {
        assert_eq!(React::parse(react).map(React::as_str), Ok(react));
    }
}
