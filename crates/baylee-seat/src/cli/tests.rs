//! The CLI mind's parts without a process: the Claude Code dialect on
//! lines of its stream-json, and the environment and program policy. The
//! mind against a stand-in process is `tests/cli_mind.rs`.

use super::claude::Claude;
use super::dialect::{Dialect, Event, Outcome, Started};
use super::*;
use crate::llm::Spec;

fn read(lines: &[&str]) -> Vec<Event> {
    let mut trouble = None;
    lines
        .iter()
        .map(|line| Claude.read_event(line, &mut trouble))
        .collect()
}

fn settings() -> Settings {
    Settings::new(&Spec::parse("cli:claude:opus").unwrap().unwrap())
}

/// What a locked-down Claude Code says at its start.
fn locked_down() -> Started {
    Started {
        tools: Some(vec!["StructuredOutput".into()]),
        mcp_servers: Some(Vec::new()),
        slash_commands: Some(Vec::new()),
        key_source: Some("none".into()),
    }
}

/// What the `init` line `fields` says, read.
fn init(fields: &Value) -> Started {
    let mut line = json!({"type": "system", "subtype": "init"});
    line.as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    let Event::Started(started) = &read(&[&line.to_string()])[0] else {
        panic!("{line}");
    };
    started.clone()
}

/// An answer comes from `structured_output`, else from the object in the
/// text; its tokens are the tool's own count, cache reads included.
#[test]
fn a_result_is_the_structured_answer_or_the_object_in_its_text() {
    let events = read(&[
        r#"{"type":"system","subtype":"init","tools":["StructuredOutput"],"mcp_servers":[],"slash_commands":[],"apiKeySource":"none"}"#,
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"hm"}]}}"#,
        r#"{"type":"result","subtype":"success","is_error":false,"result":"done","structured_output":{"ask":"q3","pick":["p"]},"usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":7,"cache_read_input_tokens":900}}"#,
        r#"{"type":"result","subtype":"success","is_error":false,"result":"Here: {\"ask\":\"q4\",\"pick\":[\"a1\"]}"}"#,
        "not json at all",
    ]);
    assert_eq!(events[0], Event::Started(locked_down()));
    assert_eq!(events[1], Event::Other);
    assert_eq!(
        events[2],
        Event::Reply(Outcome::Answer {
            value: Some(json!({"ask": "q3", "pick": ["p"]})),
            text: "done".into(),
            usage: Some(Usage {
                input: 10,
                output: 5,
                cache_write: 7,
                cache_read: 900,
            }),
        })
    );
    let Event::Reply(Outcome::Answer { value, usage, .. }) = &events[3] else {
        panic!("{:?}", events[3]);
    };
    assert_eq!(value, &Some(json!({"ask": "q4", "pick": ["a1"]})));
    assert_eq!(
        *usage, None,
        "a result that does not say counts at its worst"
    );
    assert_eq!(events[4], Event::Other);
}

/// A rate limit is said by the `assistant` line before its result, or by
/// the result's own words, and lifts when the text says.
#[test]
fn a_rate_limit_is_unbilled_and_says_when_it_lifts() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let limit = format!("Claude AI usage limit reached|{}", now + 600);
    let events = read(&[
        &json!({"type": "assistant", "error": "rate_limit", "message": {}}).to_string(),
        &json!({"type": "result", "is_error": true, "result": limit}).to_string(),
        &json!({"type": "result", "is_error": true, "result": "429 Too Many Requests"}).to_string(),
        &json!({"type": "assistant", "error": "authentication_failed", "message": {}}).to_string(),
        &json!({"type": "result", "is_error": true, "result": "Invalid API key"}).to_string(),
        &json!({"type": "result", "subtype": "error_during_execution", "is_error": false})
            .to_string(),
    ]);
    let Event::Reply(Outcome::RateLimited { why, lifts_in }) = &events[1] else {
        panic!("{:?}", events[1]);
    };
    assert_eq!(why, "rate_limit: Claude AI usage limit reached");
    let lifts_in = lifts_in.expect("the time it lifts").as_secs();
    assert!((590..=600).contains(&lifts_in), "{lifts_in}");
    assert!(
        matches!(
            &events[2],
            Event::Reply(Outcome::RateLimited { lifts_in: None, .. })
        ),
        "{:?}",
        events[2]
    );
    let Event::Reply(Outcome::Failed(why)) = &events[4] else {
        panic!("{:?}", events[4]);
    };
    assert!(why.contains("not signed in"), "{why}");
    assert_eq!(
        events[5],
        Event::Reply(Outcome::Failed(
            "the reply failed (error_during_execution)".into()
        ))
    );
}

/// The process may offer the model the answer's own tool and nothing else:
/// a tool, an MCP server or a slash command beyond it is refused, by name,
/// and so is a start that does not name its tools, servers or commands
/// (only a list it named shows the flags held), or a key from a variable.
#[test]
fn a_process_with_a_tool_beyond_the_answer_is_refused() {
    assert_eq!(Claude.lockdown_fault(&locked_down()), None);
    let unsaid = Claude.lockdown_fault(&Started::default()).unwrap();
    assert!(unsaid.contains("did not say which tools"), "{unsaid}");
    let bash = Started {
        tools: Some(vec![
            "StructuredOutput".into(),
            "Bash".into(),
            "Read".into(),
        ]),
        ..locked_down()
    };
    let why = Claude.lockdown_fault(&bash).unwrap();
    assert!(why.contains("(Bash, Read)"), "{why}");

    let fault = |fields: Value| Claude.lockdown_fault(&init(&fields));
    let all = json!({"tools": ["StructuredOutput"], "mcp_servers": [], "slash_commands": [],
                     "apiKeySource": "none"});
    assert_eq!(fault(all.clone()), None);
    let without = |key: &str| {
        let mut fields = all.clone();
        fields.as_object_mut().unwrap().remove(key);
        fault(fields).unwrap()
    };
    assert!(without("tools").contains("did not say which tools"));
    assert!(without("mcp_servers").contains("did not say which MCP servers"));
    assert!(without("slash_commands").contains("did not say which slash commands"));
    let with = |key: &str, value: Value| {
        let mut fields = all.clone();
        fields[key] = value;
        fault(fields)
    };
    let mcp = with(
        "mcp_servers",
        json!([{"name": "github", "status": "connected"}]),
    )
    .unwrap();
    assert!(mcp.contains("(github)"), "{mcp}");
    let odd = with("tools", json!(["StructuredOutput", {"kind": "tool"}])).unwrap();
    assert!(odd.contains("(unnamed)"), "{odd}");
    let commands = with("slash_commands", json!(["compact", "review"])).unwrap();
    assert!(commands.contains("(compact, review)"), "{commands}");
    let keyed = with("apiKeySource", json!("ANTHROPIC_API_KEY")).unwrap();
    assert!(keyed.contains("a key from ANTHROPIC_API_KEY"), "{keyed}");
    for login in ["none", "/login managed key", "apiKeyHelper"] {
        assert_eq!(with("apiKeySource", json!(login)), None, "{login}");
    }
    let mut unsourced = all.clone();
    unsourced.as_object_mut().unwrap().remove("apiKeySource");
    assert_eq!(fault(unsourced), None, "a start that names no key's source");
}

/// The arguments lock the process down, and carry our instructions, our
/// schema, the model and the effort.
#[test]
fn claude_runs_with_no_tools_no_settings_and_our_instructions() {
    let mut settings = settings();
    settings.effort = Some("low".into());
    let args: Vec<String> = Claude
        .args(&settings, Some("opus"), "SYSTEM")
        .into_iter()
        .map(|arg| arg.into_string().unwrap())
        .collect();
    let schema = prompt::answer_schema().to_string();
    let expected = [
        "-p",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
        "--restricted",
        "--safe-mode",
        "--tools",
        "",
        "--strict-mcp-config",
        "--disable-slash-commands",
        "--setting-sources",
        "",
        "--permission-prompts",
        "none",
        "--permission-mode",
        "manual",
        "--no-session-persistence",
        "--system-prompt",
        "SYSTEM",
        "--json-schema",
        &schema,
        "--model",
        "opus",
        "--effort",
        "low",
    ];
    assert_eq!(args, expected);
    assert!(
        !args
            .iter()
            .any(|arg| arg == "--bare" || arg.contains("dangerously"))
    );
    let line: Value = serde_json::from_str(&Claude.stdin_line("q1 ·")).unwrap();
    assert_eq!(
        line,
        json!({"type": "user", "message": {"role": "user", "content": "q1 ·"}})
    );
    assert!(Claude.probe_ok(br#"{"loggedIn": true}"#));
    assert!(!Claude.probe_ok(br#"{"loggedIn": false}"#));
    // A status it cannot read is no login.
    assert!(!Claude.probe_ok(b"Logged in as someone"));
    assert!(!Claude.probe_ok(br#"{"authMethod": "claude.ai"}"#));
}

/// No key, token, bridge setting, cloud or forge credential, SSH agent or
/// database ever reaches a CLI, and a passed value shaped like a key
/// refuses the start without being shown.
#[test]
fn the_environment_passes_by_name_and_never_a_key() {
    for name in [
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
        "GEMINI_API_KEY",
        "DEEPSEEK_API_KEY",
        "ANTHROPIC_BASE_URL",
        "GITHUB_PERSONAL_ACCESS_TOKEN",
        "GH_TOKEN",
        "BAYLEE_SEAT_CONFIG",
        "BAYLEE_LLM_API_KEY",
        "AWS_SECRET_ACCESS_KEY",
        "SSH_AUTH_SOCK",
        "DATABASE_URL",
        "NPM_TOKEN",
        "CLIENT_SECRET",
    ] {
        assert!(forbidden(name), "{name}");
    }
    for name in COMMON
        .iter()
        .chain(Claude.passed_env())
        .chain(Claude.fixed_env().iter().map(|(name, _)| name))
        .chain(["TMPDIR", "LANG", "LC_ALL", "TERM", "NO_COLOR"].iter())
    {
        assert!(!forbidden(name), "{name}");
    }
    assert!(key_shaped("x sk-ant-api03-AAAABBBBCCCCDDDD y"));
    assert!(key_shaped("ghp_0123456789abcdefghijABCD"));
    assert!(key_shaped("Bearer abc"));
    assert!(!key_shaped("/Users/someone"));
    assert!(!key_shaped("/usr/local/bin:/usr/bin:/bin"));

    let home = "/Users/sk-ant-api03-0123456789abcdefghij";
    let env = |name: &str| match name {
        "HOME" => Some(home.to_string()),
        "PATH" => Some("/nowhere".to_string()),
        _ => None,
    };
    let refused = Launch::new(&settings(), Some("/bin/sh"), &env).unwrap_err();
    assert!(refused.contains("HOME looks like a key"), "{refused}");
    assert!(!refused.contains("0123456789"), "{refused}");
}

/// A program is a whole path, or the tool's name on the absolute entries
/// of `PATH`; never a shell's function or alias, and never a relative
/// entry that would depend on the working directory.
#[test]
fn the_program_is_a_whole_path_or_found_on_path() {
    let none = |_: &str| None;
    let relative = program(CliTool::Claude, Some("bin/claude"), &none).unwrap_err();
    assert!(relative.contains("whole path"), "{relative}");
    let missing = program(CliTool::Claude, Some("/nowhere/claude"), &none).unwrap_err();
    assert!(missing.contains("not a program"), "{missing}");
    let not_found = program(CliTool::Claude, None, &|name: &str| {
        (name == "PATH").then(|| ".:bin:/nowhere".to_string())
    })
    .unwrap_err();
    assert!(not_found.contains("not on PATH"), "{not_found}");
}
