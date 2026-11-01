//! Antigravity argument and stream fixtures.

use super::*;
use crate::llm::Spec;

/// An old CLI or an ignored agent flag must fail before an answer is used.
#[test]
fn agy_requires_the_restricted_agent_at_start() {
    for agent in [Value::Null, json!("default"), json!("another-agent")] {
        let line = json!({"event":"init", "init": {"agent":agent, "tools":["finish"]}});
        let event = super::super::agy::Agy.read_event(&line.to_string(), &mut Wire::default());
        assert!(
            matches!(event, Event::Breach(why) if why.contains("restricted baylee-seat agent"))
        );
    }
}

/// The Agy dialect: arguments, stdin format, event parsing, and probes.
#[test]
fn agy_dialect_args_and_events() {
    use super::super::agy::Agy;

    assert_eq!(Agy.tool(), CliTool::Agy);
    assert_eq!(Agy.passed_env(), &[] as &[&str]);
    assert_eq!(Agy.fixed_env(), &[] as &[(&str, &str)]);
    assert!(Agy.probe_ok(b"1.2.15\n", b""));
    assert!(!Agy.probe_ok(b"", b""));

    let settings = Settings::new(
        &Spec::parse("cli:agy:gemini-3.8-flash-high")
            .unwrap()
            .unwrap(),
    );
    let args: Vec<String> = Agy
        .args(
            &settings,
            Some("gemini-3.8-flash-high"),
            "system",
            Path::new("/support"),
        )
        .into_iter()
        .map(|a| a.to_str().unwrap().to_string())
        .collect();
    assert!(args.contains(&"--input-format".to_string()));
    assert!(args.contains(&"stream-json".to_string()));
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"gemini-3.8-flash-high".to_string()));
    // The model plays through stdin and stdout only: it is never handed
    // permission to run tools (ad6c379a dropped the flag; the test had not
    // followed).
    assert!(!args.iter().any(|a| a.contains("dangerously")));

    // Tier-suffixed agy models reconcile their suffix with `--effort` so
    // `agy` never rejects a conflicting `--model ...-high --effort low`.
    let mut low_settings = settings.clone();
    low_settings.effort = Some("low".into());
    let low_args: Vec<String> = Agy
        .args(
            &low_settings,
            Some("gemini-3.8-flash-high"),
            "system",
            Path::new("/support"),
        )
        .into_iter()
        .map(|a| a.to_str().unwrap().to_string())
        .collect();
    assert!(low_args.contains(&"gemini-3.8-flash-low".to_string()));
    assert!(low_args.contains(&"low".to_string()));

    // Instructions are in the restricted agent, never guessed from game text.
    let line1 = Agy.stdin_line("THE GAME\nYou are P1 at a table");
    let val1: Value = serde_json::from_str(&line1).unwrap();
    assert_eq!(val1["event"], "user");
    assert_eq!(
        val1["message"]["content"],
        "THE GAME\nYou are P1 at a table"
    );
    assert!(
        val1["message"]["content"]
            .as_str()
            .unwrap()
            .contains("THE GAME\nYou are P1")
    );

    let line2 = Agy.stdin_line("q2: choose option a1");
    let val2: Value = serde_json::from_str(&line2).unwrap();
    assert_eq!(val2["event"], "user");
    assert_eq!(val2["message"]["content"], "q2: choose option a1");

    // Event parsing: init
    let mut trouble = None;
    let init_line = json!({
        "event": "init",
        "init": {
            "agent": "baylee-seat",
            "model": "gemini-3.8-flash-high",
            "tools": ["finish", "run_command"]
        }
    })
    .to_string();
    let ev1 = read_with(&Agy, &init_line, &mut trouble);
    assert!(matches!(ev1, Event::Started(_)));

    // Event parsing: successful result
    let result_line = json!({
        "event": "result",
        "result": {
            "status": "SUCCESS",
            "response": "{\"ask\":\"q1\",\"pick\":[\"a1\"]}",
            "structured_output": {"ask": "q1", "pick": ["a1"]},
            "usage": {
                "input_tokens": 100,
                "output_tokens": 20,
                "cache_read_tokens": 50
            }
        }
    })
    .to_string();
    let ev2 = read_with(&Agy, &result_line, &mut trouble);
    let Event::Reply(Outcome::Answer { value, usage, .. }) = ev2 else {
        panic!("expected answer");
    };
    assert_eq!(value.unwrap()["ask"], "q1");
    assert_eq!(usage.unwrap().input, 100);

    // Event parsing: rate limited
    let rate_limit_line = json!({
        "event": "result",
        "result": {
            "status": "ERROR",
            "error": "RESOURCE_EXHAUSTED: quota exceeded"
        }
    })
    .to_string();
    let ev3 = read_with(&Agy, &rate_limit_line, &mut trouble);
    assert!(matches!(ev3, Event::Reply(Outcome::RateLimited { .. })));
}
