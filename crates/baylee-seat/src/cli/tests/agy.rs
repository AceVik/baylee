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

/// agy's login check is `/quota` in print mode, which calls no model: a
/// `SUCCESS` that names a group of models is signed in; a version, an
/// error, a quota without groups or output it cannot read is no login.
#[test]
fn agy_login_check_is_its_quota() {
    use super::super::agy::Agy;

    assert_eq!(
        Agy.probe_args().unwrap(),
        ["-p", "/quota", "--output-format", "json"].map(std::ffi::OsString::from)
    );
    assert!(Agy.probe_ok(QUOTA_1_5_18.as_bytes(), b""));
    for no in [
        "1.5.18\n",
        "",
        r#"{"status":"ERROR","error":"not signed in","command":null}"#,
        r#"{"status":"SUCCESS","command":{"name":"usage","data":{"groups":[]}}}"#,
    ] {
        assert!(!Agy.probe_ok(no.as_bytes(), b""), "{no}");
    }
}

/// The Agy dialect: arguments, stdin format and event parsing.
#[test]
fn agy_dialect_args_and_events() {
    use super::super::agy::Agy;

    assert_eq!(Agy.tool(), CliTool::Agy);
    assert_eq!(Agy.passed_env(), &[] as &[&str]);
    assert_eq!(Agy.fixed_env(), &[] as &[(&str, &str)]);

    let settings = Settings::new(
        &Spec::parse("cli:agy:gemini-3.8-flash-high")
            .unwrap()
            .unwrap(),
    );
    let args: Vec<String> = Agy
        .args(
            &settings,
            Some("gemini-3.8-flash-high"),
            &crate::cli::Brief::told(&settings, "system"),
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
            &crate::cli::Brief::told(&settings, "system"),
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

#[test]
fn agy_gemini4_and_claude_tier_reconciliation() {
    use super::super::agy::Agy;

    // Gemini 4 Hydrogen supports `-xhigh` and reconciles across all four
    // tiers (`low`, `medium`, `high`, `xhigh`), while three-tier models
    // (Argon, Carbon, Claude Opus/Sonnet 5.5) clamp `xhigh`/`max` to `high`.
    for (raw, ask_effort, want_model, want_effort) in [
        (
            "gemini-4-hydrogen-high",
            "xhigh",
            "gemini-4-hydrogen-xhigh",
            "xhigh",
        ),
        (
            "gemini-4-hydrogen-xhigh",
            "low",
            "gemini-4-hydrogen-low",
            "low",
        ),
        (
            "gemini-4-hydrogen-xhigh",
            "max",
            "gemini-4-hydrogen-xhigh",
            "xhigh",
        ),
        (
            "gemini-4-argon-medium",
            "xhigh",
            "gemini-4-argon-high",
            "high",
        ),
        (
            "gemini-4-carbon-low",
            "high",
            "gemini-4-carbon-high",
            "high",
        ),
        (
            "claude-opus-5-5-high",
            "medium",
            "claude-opus-5-5-medium",
            "medium",
        ),
        (
            "claude-sonnet-5-5-low",
            "high",
            "claude-sonnet-5-5-high",
            "high",
        ),
    ] {
        let mut s = Settings::new(&Spec::parse(&format!("cli:agy:{raw}")).unwrap().unwrap());
        s.effort = Some(ask_effort.into());
        let got: Vec<String> = Agy
            .args(
                &s,
                Some(raw),
                &crate::cli::Brief::told(&s, "system"),
                Path::new("/support"),
            )
            .into_iter()
            .map(|a| a.to_str().unwrap().to_string())
            .collect();
        let m_idx = got.iter().position(|a| a == "--model").unwrap();
        let e_idx = got.iter().position(|a| a == "--effort").unwrap();
        assert_eq!(got[m_idx + 1], want_model, "{raw} + {ask_effort}");
        assert_eq!(got[e_idx + 1], want_effort, "{raw} + {ask_effort}");
    }
}

/// `agy -p /quota --output-format json` as agy 1.5.18 answered it on
/// 27.11.2026, shortened to one group: the login check's proof.
const QUOTA_1_5_18: &str = r#"{"conversation_id": "", "status": "SUCCESS",
  "response": "Gemini Models\tWeekly Limit Remaining\t69%\t2026-12-03T07:02:07Z\n",
  "duration_seconds": 0, "num_turns": 0,
  "usage": {"input_tokens": 0, "output_tokens": 0, "thinking_tokens": 0,
            "cache_read_tokens": 0, "total_tokens": 0},
  "command": {"name": "usage", "data": {
    "description": "Within each group, models share a weekly limit and a 5-hour limit.",
    "groups": [{"name": "Gemini Models", "buckets": [{"id": "gemini-weekly",
      "name": "Weekly Limit Remaining", "window": "weekly",
      "remaining_fraction": 0.69, "reset_time": "2026-12-03T07:02:07Z"}]}]}}}"#;

/// agy's arguments for `model` at `effort`.
fn agy_args(model: &str, effort: Option<&str>) -> Vec<String> {
    use super::super::agy::Agy;
    let mut settings = Settings::new(&Spec::parse(&format!("cli:agy:{model}")).unwrap().unwrap());
    settings.effort = effort.map(str::to_string);
    Agy.args(
        &settings,
        Some(model),
        &crate::cli::Brief::told(&settings, "system"),
        Path::new("/support"),
    )
    .into_iter()
    .map(|a| a.to_str().unwrap().to_string())
    .collect()
}

/// The value after `flag`, if it is passed.
fn after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let at = args.iter().position(|a| a == flag)?;
    args.get(at + 1).map(String::as_str)
}

/// Gemini 4 Neon goes up to `xhigh`, as Hydrogen does (`agy models`,
/// 1.5.18): its xhigh is never played as high.
#[test]
fn agy_neon_keeps_its_xhigh_tier() {
    for (raw, effort, model, played) in [
        (
            "gemini-4-neon-xhigh",
            Some("xhigh"),
            "gemini-4-neon-xhigh",
            Some("xhigh"),
        ),
        (
            "gemini-4-neon-high",
            Some("xhigh"),
            "gemini-4-neon-xhigh",
            Some("xhigh"),
        ),
        (
            "gemini-4-neon-xhigh",
            Some("max"),
            "gemini-4-neon-xhigh",
            Some("xhigh"),
        ),
        (
            "gemini-4-neon-xhigh",
            Some("low"),
            "gemini-4-neon-low",
            Some("low"),
        ),
        (
            "gemini-4-neon",
            Some("xhigh"),
            "gemini-4-neon-xhigh",
            Some("xhigh"),
        ),
        ("gemini-4-neon-xhigh", None, "gemini-4-neon-xhigh", None),
    ] {
        let args = agy_args(raw, effort);
        assert_eq!(after(&args, "--model"), Some(model), "{raw} {effort:?}");
        assert_eq!(after(&args, "--effort"), played, "{raw} {effort:?}");
    }
}

/// agy refuses `--effort` for its orchestrators ("--effort is not supported
/// for model", 1.5.18): the seat never passes one, whatever the profile
/// names, so the game starts and plays agy's own orchestrator setting.
#[test]
fn agy_orchestrators_are_started_without_an_effort() {
    for model in [
        "gemini-pro-orchestrator-neon",
        "gemini-pro-orchestrator",
        "gemini-pro-orchestrator-argon",
    ] {
        for effort in [None, Some("xhigh"), Some("high")] {
            let args = agy_args(model, effort);
            assert_eq!(after(&args, "--model"), Some(model), "{effort:?}");
            assert!(!args.iter().any(|a| a == "--effort"), "{model} {effort:?}");
            assert!(args.iter().any(|a| a == "--agent"), "{model}");
        }
    }
}

/// The private agent lists only `finish` and drops agy's own prompt
/// sections and built-in tools (1.5.18: 470 input tokens on a first model
/// step where 11,029 without).
#[test]
fn agy_agent_file_is_finish_only_and_lean() {
    use super::super::agy::Agy;
    let settings = Settings::new(
        &Spec::parse("cli:agy:gemini-3.8-flash-low")
            .unwrap()
            .unwrap(),
    );
    let files = Agy.workspace_files(&crate::cli::Brief::told(&settings, "OUR SYSTEM"));
    let [(path, text)] = files.as_slice() else {
        panic!("one agent file: {files:?}");
    };
    assert_eq!(*path, ".agents/agents/baylee-seat.md");
    let front = text
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .map(|(front, _)| front)
        .expect("front matter");
    for line in [
        "name: baylee-seat",
        "tools: [finish]",
        "subagent: false",
        "commandExecutionPolicy: off",
        "mcpServers: []",
        "skills: []",
        "plugins: []",
        "excludeDefaultComponents: true",
    ] {
        assert!(front.lines().any(|l| l == line), "{line} in {front}");
    }
    assert!(text.contains("OUR SYSTEM"));
}

/// agy 1.5.18 offers every agent `manage_task` beside `finish`, whatever its
/// `tools` list says; a step that uses it, or any tool but `finish` (here
/// the `read_file` a `view_file` agent ran, as recorded), takes the mind off
/// the table. `finish` and a `finish` step are the answer's own.
#[test]
fn agy_manage_task_or_any_tool_but_finish_is_a_breach() {
    use super::super::agy::Agy;
    let step = |state: &str, kind: &str, tool: Option<&str>| {
        let mut step = json!({
            "conversation_id": "d1296a87-6e58-4a6d-b6ff-eb89b0d67f64",
            "step_index": 2,
            "state": state,
            "step_type": kind,
        });
        if let Some(tool) = tool {
            step["tool_name"] = json!(tool);
            step["tool_info"] = json!({"name": tool, "parameters": {}});
        }
        json!({"event": "step_update", "step_update": step}).to_string()
    };
    for tool in [
        "manage_task",
        "read_file",
        "run_command",
        "run_workflow",
        "invoke_subagent",
    ] {
        for state in ["ACTIVE", "DONE"] {
            let event = Agy.read_event(&step(state, "tool", Some(tool)), &mut Wire::default());
            assert!(
                matches!(&event, Event::Breach(why) if why.contains(tool)),
                "{tool} {state}: {event:?}"
            );
        }
    }
    // A tool step that names its tool only at its top is still one.
    let bare = json!({"event": "step_update", "step_update": {
        "state": "ACTIVE", "step_type": "tool", "tool_name": "manage_task"}})
    .to_string();
    assert!(matches!(
        Agy.read_event(&bare, &mut Wire::default()),
        Event::Breach(why) if why.contains("manage_task")
    ));
    assert!(matches!(
        Agy.read_event(&step("DONE", "tool", Some("finish")), &mut Wire::default()),
        Event::Other
    ));
    assert!(matches!(
        Agy.read_event(&step("DONE", "finish", None), &mut Wire::default()),
        Event::Other
    ));
}

/// A result agy cut short (`finished_reason`, 1.4.11) is no answer, even
/// with a response that reads as one; a completed result says null.
#[test]
fn agy_a_cut_off_result_is_no_answer() {
    use super::super::agy::Agy;
    let result = |reason: Value| {
        json!({"event": "result", "result": {
            "status": "SUCCESS",
            "response": "{\"answer\":\"x\"}",
            "structured_output": {"answer": "x"},
            "finished_reason": reason,
            "usage": {"input_tokens": 470, "output_tokens": 2580, "thinking_tokens": 2553,
                      "cache_read_tokens": 0, "total_tokens": 3050}
        }})
        .to_string()
    };
    let Event::Reply(Outcome::Answer { value, usage, .. }) =
        Agy.read_event(&result(Value::Null), &mut Wire::default())
    else {
        panic!("a completed turn is an answer");
    };
    assert_eq!(value.unwrap()["answer"], "x");
    let usage = usage.unwrap();
    // `output_tokens` already holds the thinking (470 + 2580 = 3050).
    assert_eq!((usage.input, usage.output), (470, 2580));
    for reason in ["print_timeout", "interrupted", "error"] {
        let event = Agy.read_event(&result(json!(reason)), &mut Wire::default());
        assert!(
            matches!(&event, Event::Reply(Outcome::Failed(why)) if why.contains(reason)),
            "{reason}: {event:?}"
        );
    }
}
