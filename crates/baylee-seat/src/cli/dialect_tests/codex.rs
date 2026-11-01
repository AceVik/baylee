//! Codex argument, event and login fixtures.

use super::*;

// ---------------------------------------------------------------------
// Codex
// ---------------------------------------------------------------------

/// `codex exec`, locked down: user config, rules and project docs off,
/// our instructions and schema by file, every tool feature off, read-only,
/// no prompt history, the prompt from stdin; model and effort where named.
// Unix only: unix paths and a home found by HOME, as the CLI seat has
// them (Windows sets no HOME and joins with \; not yet supported there).
#[cfg(unix)]
#[test]
fn codex_runs_exec_with_our_instructions_and_no_tools() {
    let args = args_of(&Codex, Some("gpt-6.1-sol"), Some("low"));
    let mut expected: Vec<String> = [
        "exec",
        "--json",
        "--color",
        "never",
        "--ignore-user-config",
        "--ignore-rules",
        "--skip-git-repo-check",
        "--sandbox",
        "read-only",
        "--output-schema",
        "/s/schema.json",
    ]
    .map(str::to_string)
    .into();
    for config in [
        "model_instructions_file=\"/s/instructions.md\"",
        "project_doc_max_bytes=0",
        "mcp_servers={}",
        "notify=[]",
        "web_search=\"disabled\"",
        "tools.view_image=false",
        "history.persistence=\"none\"",
        "analytics.enabled=false",
        "feedback.enabled=false",
        "otel.exporter=\"none\"",
        "check_for_update_on_startup=false",
        "include_environment_context=false",
        "include_permissions_instructions=false",
        "include_apps_instructions=false",
        "model_reasoning_summary=\"none\"",
        "model_reasoning_effort=\"low\"",
    ] {
        expected.push("-c".into());
        expected.push(config.into());
    }
    for feature in DISABLED_FEATURES {
        expected.push("--disable".into());
        expected.push(feature.into());
    }
    expected.extend(["--model", "gpt-6.1-sol", "-"].map(str::to_string));
    assert_eq!(args, expected);
    for feature in [
        "shell_tool",
        "unified_exec",
        "plugins",
        "hooks",
        "multi_agent",
    ] {
        assert!(DISABLED_FEATURES.contains(&feature), "{feature}");
    }
    let bare = args_of(&Codex, None, None);
    assert!(!bare.iter().any(|a| a == "--model" || a.contains("effort")));
    assert_eq!(bare.last().map(String::as_str), Some("-"));
    // The files the flags name: ours, whole.
    let files = Codex.files(&settings_for("codex", None), "SYSTEM");
    assert_eq!(files[0], ("instructions.md", "SYSTEM".to_string()));
    assert_eq!(files[1].0, "schema.json");
    let schema: Value = serde_json::from_str(&files[1].1).unwrap();
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["ask"],
        prompt::answer_schema(prompt::Extras::default())["properties"]["ask"]
    );
    assert!(Codex.one_shot());
    assert_eq!(Codex.stdin_line("q1\nline two"), "q1\nline two");
    // A path with a quote or a backslash stays one TOML string.
    let odd = Codex.args(
        &settings_for("codex", None),
        None,
        "S",
        Path::new("/a\"b\\c"),
    );
    let odd: Vec<String> = odd.into_iter().map(|a| a.into_string().unwrap()).collect();
    assert!(odd.contains(&r#"model_instructions_file="/a\"b\\c/instructions.md""#.to_string()));
}

/// Codex's events as its non-interactive docs show them: the thread's
/// start is the start, the answer is the agent message's text, the turn's
/// usage has its cached tokens taken out of its input.
#[test]
fn codex_reads_its_answer_and_usage_off_the_turn() {
    let events = read_all(
        &Codex,
        &[
            r#"{"type":"thread.started","thread_id":"0199a213-81c0-7800-8aa1-bbab2a035a53"}"#,
            r#"{"type":"turn.started"}"#,
            r#"{"type":"item.completed","item":{"id":"item_0","type":"reasoning","text":"hm"}}"#,
            r#"{"type":"item.completed","item":{"id":"item_3","type":"agent_message","text":"{\"ask\":\"q1\",\"pick\":[\"a\"]}"}}"#,
            r#"{"type":"turn.completed","usage":{"input_tokens":24763,"cached_input_tokens":24448,"output_tokens":122,"reasoning_output_tokens":0}}"#,
        ],
    );
    assert_eq!(events[0], Event::Started(Started::default()));
    assert_eq!(events[1..5], [const { Event::Other }; 4]);
    let Event::Reply(Outcome::Answer { value, text, usage }) = &events[5] else {
        panic!("{:?}", events[5]);
    };
    assert_eq!(value.as_ref().unwrap()["ask"], "q1");
    assert!(text.contains("pick"));
    let usage = usage.unwrap();
    assert_eq!(
        (
            usage.input,
            usage.cache_read,
            usage.output,
            usage.cache_write
        ),
        (315, 24448, 122, 0)
    );
    assert!(Codex.usage_is_cumulative());
    assert_eq!(Codex.lockdown_fault(&Started::default()), None);
}

/// Any item but the answer, the reasoning and a warning is a tool used:
/// the lockdown is broken, whether it began or finished.
#[test]
fn codex_takes_a_tool_item_as_a_breach() {
    for kind in [
        "command_execution",
        "file_change",
        "mcp_tool_call",
        "web_search",
        "todo_list",
    ] {
        for event in ["item.started", "item.completed"] {
            let line = json!({"type": event, "item": {"id": "i", "type": kind}}).to_string();
            let read = read_all(&Codex, &[&line]);
            let Event::Breach(why) = &read[1] else {
                panic!("{kind}: {read:?}");
            };
            assert!(why.contains(kind), "{why}");
        }
    }
    let warning = r#"{"type":"item.completed","item":{"id":"i","type":"error","message":"x"}}"#;
    assert_eq!(
        read_all(&Codex, &[warning]),
        [Event::Started(Started::default()), Event::Other]
    );
}

/// A failed turn is a limit by its words, else a failure; an `error` line
/// before it (a retry Codex makes itself) ends nothing and names the
/// failure that has no words of its own.
#[test]
fn codex_reads_a_failed_turn_as_a_limit_or_a_failure() {
    let limited = read_all(
        &Codex,
        &[
            r#"{"type":"thread.started","thread_id":"t"}"#,
            r#"{"type":"error","message":"Reconnecting... 1/5"}"#,
            r#"{"type":"turn.failed","error":{"message":"You've hit your usage limit. Try again at 3:04 PM."}}"#,
        ],
    );
    assert_eq!(limited[2], Event::Other, "a retry ends nothing");
    assert!(matches!(
        &limited[3],
        Event::Reply(Outcome::RateLimited { lifts_in: None, .. })
    ));
    let failed = read_all(
        &Codex,
        &[
            r#"{"type":"error","message":"stream disconnected"}"#,
            r#"{"type":"turn.failed","error":{}}"#,
            r#"{"type":"turn.failed","error":{}}"#,
        ],
    );
    assert_eq!(
        failed[2],
        Event::Reply(Outcome::Failed("stream disconnected".into()))
    );
    assert_eq!(
        failed[3],
        Event::Reply(Outcome::Failed("the turn failed".into())),
        "the next turn does not inherit it"
    );
    let retries = read_all(
        &Codex,
        &[
            r#"{"type":"turn.failed","error":{"message":"exceeded retry limit, last status: 429 Too Many Requests"}}"#,
        ],
    );
    assert!(matches!(
        retries[1],
        Event::Reply(Outcome::RateLimited { .. })
    ));
}

/// Signed in to a subscription only where `codex login status` says so on
/// stderr; a key's login, or none, is not.
#[test]
fn codex_is_signed_in_by_its_status_on_stderr() {
    assert_eq!(
        Codex.probe_args().unwrap(),
        [OsString::from("login"), OsString::from("status")]
    );
    assert!(Codex.probe_ok(b"", b"Logged in using ChatGPT\n"));
    assert!(!Codex.probe_ok(b"", b"Logged in using an API key - sk-***\n"));
    assert!(!Codex.probe_ok(b"", b"Not logged in\n"));
    assert!(!Codex.probe_ok(b"", b""));
}
