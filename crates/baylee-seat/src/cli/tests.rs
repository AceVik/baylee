//! The CLI mind's parts without a process: the Claude Code dialect on
//! lines of its stream-json, and the environment and program policy. The
//! mind against a stand-in process is `tests/cli_mind.rs`.

use super::claude::Claude;
use super::dialect::{Dialect, Event, Outcome, Started, Wire};
use super::*;
use crate::llm::Spec;

/// One line read by `dialect`, `trouble` carried in and out as a reply's
/// lines carry it.
fn read_with(dialect: &dyn Dialect, line: &str, trouble: &mut Option<String>) -> Event {
    let mut wire = Wire {
        trouble: trouble.take(),
        ..Wire::default()
    };
    let event = dialect.read_event(line, &mut wire);
    *trouble = wire.trouble;
    event
}

fn read(lines: &[&str]) -> Vec<Event> {
    let mut trouble = None;
    lines
        .iter()
        .map(|line| read_with(&Claude, line, &mut trouble))
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
                ..Usage::default()
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
    assert!(without("apiKeySource").contains("did not say where its key comes from"));
}

/// The arguments lock the process down, and carry our instructions, our
/// schema, the model and the effort.
#[test]
fn claude_runs_with_no_tools_no_settings_and_our_instructions() {
    let mut settings = settings();
    settings.effort = Some("low".into());
    let args: Vec<String> = Claude
        .args(&settings, Some("opus"), "SYSTEM", Path::new("/support"))
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
    assert!(Claude.probe_ok(br#"{"loggedIn": true}"#, b""));
    assert!(!Claude.probe_ok(br#"{"loggedIn": false}"#, b""));
    // A status it cannot read is no login.
    assert!(!Claude.probe_ok(b"Logged in as someone", b""));
    assert!(!Claude.probe_ok(br#"{"authMethod": "claude.ai"}"#, b""));
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
    assert!(shaped_like_a_key("x sk-ant-api03-AAAABBBBCCCCDDDD y"));
    assert!(shaped_like_a_key(
        "/Users/sk-ant-api03-0123456789abcdefghij"
    ));
    assert!(shaped_like_a_key("ghp_0123456789abcdefghijABCD"));
    assert!(shaped_like_a_key("TOKEN=github_pat_0123456789abcdefghij_x"));
    assert!(shaped_like_a_key("Bearer abc"));
    assert!(!shaped_like_a_key("/Users/someone"));
    assert!(!shaped_like_a_key("/usr/local/bin:/usr/bin:/bin"));
    // A marker inside a word is no key: `desk-tools-collection` holds
    // `sk-` and sixteen key characters after it.
    let desk = "/opt/desk-tools-collection/bin:/usr/bin";
    assert!(!shaped_like_a_key(desk));
    let tool = std::env::current_exe().unwrap();
    let tool = tool.to_str().unwrap();
    let launch = Launch::new(&settings(), Some(tool), &|name: &str| {
        (name == "PATH").then(|| desk.to_string())
    });
    assert!(launch.is_ok(), "{launch:?}");

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
/// entry that would depend on the working directory, even one that holds
/// the tool.
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
    #[cfg(unix)]
    a_relative_entry_holding_the_tool_is_skipped();
}

/// A `claude` in a directory named on `PATH` both relatively (from the
/// test's working directory, up to the root and down again) and whole:
/// only the whole entry finds it.
#[cfg(unix)]
fn a_relative_entry_holding_the_tool_is_skipped() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("baylee-cli-program-{}", std::process::id()));
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let tool = bin.join("claude");
    std::fs::write(&tool, "#!/bin/sh\n").unwrap();
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
    let here = std::env::current_dir().unwrap();
    let up = here
        .components()
        .filter(|part| matches!(part, std::path::Component::Normal(_)))
        .count();
    let relative = std::iter::repeat_n("..", up)
        .collect::<PathBuf>()
        .join(bin.strip_prefix("/").unwrap());
    assert!(relative.is_relative());
    assert!(runnable(&relative.join("claude")), "the entry holds it");
    let find = |path: String| {
        program(CliTool::Claude, None, &move |name: &str| {
            (name == "PATH").then(|| path.clone())
        })
    };
    let skipped = find(format!("{}:/nowhere", relative.display())).unwrap_err();
    assert!(skipped.contains("not on PATH"), "{skipped}");
    let found = find(format!("{}:{}", relative.display(), bin.display()));
    assert_eq!(found, Ok(tool));
    std::fs::remove_dir_all(&root).unwrap();
}

/// The time a rate limit names is believed only within reason: a past
/// time, none at all, or more than fifteen minutes is as if it named none,
/// and the mind cools for the doubling cooldown's step instead.
#[test]
fn a_limits_named_time_is_believed_up_to_fifteen_minutes() {
    let half = Duration::from_secs(30);
    assert_eq!(believed(Some(half)), Some(half));
    assert_eq!(believed(Some(MAX_COOLDOWN)), Some(MAX_COOLDOWN));
    assert_eq!(believed(Some(Duration::ZERO)), None);
    assert_eq!(believed(Some(MAX_COOLDOWN + Duration::from_secs(1))), None);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let past = format!("Claude AI usage limit reached|{}", now - 60);
    let events = read(&[&json!({"type": "result", "is_error": true, "result": past}).to_string()]);
    assert!(
        matches!(
            &events[0],
            Event::Reply(Outcome::RateLimited { lifts_in: None, .. })
        ),
        "{:?}",
        events[0]
    );

    let tool = std::env::current_exe().unwrap();
    let launch = Launch::new(&settings(), Some(tool.to_str().unwrap()), &|_: &str| None).unwrap();
    let limits = Limits {
        cooldown: Duration::from_secs(60),
        ..Limits::default()
    };
    let mind = CliMind::new(settings(), launch, limits);
    mind.cool(Some(Duration::from_hours(24)));
    let left = mind.cooling().unwrap();
    assert!(left <= limits.cooldown, "a day is not believed: {left:?}");
    mind.cool(Some(half));
    let left = mind.cooling().unwrap();
    assert!(left <= half && left > half / 2, "{left:?}");
}

// ---------------------------------------------------------------------
// The dialect, line by line
// ---------------------------------------------------------------------

fn result_line(fields: &Value) -> String {
    let mut line = json!({"type": "result"});
    line.as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    line.to_string()
}

/// The outcome of one `result` line, after `trouble` was said earlier.
fn outcome_of(trouble: Option<&str>, fields: &Value) -> Outcome {
    let mut trouble = trouble.map(str::to_string);
    match read_with(&Claude, &result_line(fields), &mut trouble) {
        Event::Reply(outcome) => outcome,
        other => panic!("{other:?}"),
    }
}

/// When the limit text says it lifts, as the mind is told.
fn lifts(text: &str) -> Option<Duration> {
    match outcome_of(
        None,
        &json!({"is_error": true, "result": format!("usage limit{text}")}),
    ) {
        Outcome::RateLimited { lifts_in, .. } => lifts_in,
        other => panic!("{other:?}"),
    }
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// A time after the last `|`, and only a number there, is believed; the
/// last bar counts, space around it is allowed, and a number too big for a
/// clock is no panic and no cooldown.
#[test]
fn a_limits_time_is_the_number_after_the_last_bar_and_nothing_else() {
    assert_eq!(lifts(""), None, "no bar");
    assert_eq!(lifts("|"), None, "nothing after it");
    assert_eq!(lifts("| soon"), None, "words");
    assert_eq!(lifts("|-5"), None, "negative");
    assert_eq!(lifts("|12.5"), None, "fraction");
    let at = unix_now() + 300;
    let spaced = lifts(&format!("  |  {at}  ")).expect("trimmed");
    assert!((290..=300).contains(&spaced.as_secs()), "{spaced:?}");
    let last = lifts(&format!("|1|{at}")).expect("the last bar counts");
    assert!((290..=300).contains(&last.as_secs()), "{last:?}");
    assert_eq!(lifts(&format!("|{at}|oops")), None, "the last bar is words");
    assert_eq!(lifts(&format!("|{}", unix_now() - 5)), None, "past");
    let huge = lifts(&format!("|{}", u64::MAX));
    assert_eq!(believed(huge), None, "no clock believes it: {huge:?}");
}

/// What the `assistant` line said is spent by the `result` that ends the
/// reply: the next reply starts clean.
#[test]
fn the_trouble_an_assistant_line_names_is_spent_by_its_result() {
    let mut trouble = None;
    let assistant = json!({"type": "assistant", "error": "server_error", "message": {}});
    assert_eq!(
        read_with(&Claude, &assistant.to_string(), &mut trouble),
        Event::Other
    );
    assert_eq!(trouble.as_deref(), Some("server_error"));
    let failing = result_line(&json!({"is_error": true, "result": "boom"}));
    let Event::Reply(Outcome::Failed(why)) = read_with(&Claude, &failing, &mut trouble) else {
        panic!();
    };
    assert_eq!(why, "server_error: boom");
    assert_eq!(trouble, None, "taken by its result");
    let Event::Reply(Outcome::Failed(why)) = read_with(&Claude, &failing, &mut trouble) else {
        panic!();
    };
    assert_eq!(why, "boom", "the next reply does not inherit it");
    // An assistant line with no error says nothing, and clears nothing.
    let mut kept = Some("rate_limit".to_string());
    let fine = json!({"type": "assistant", "message": {}});
    read_with(&Claude, &fine.to_string(), &mut kept);
    assert_eq!(kept.as_deref(), Some("rate_limit"));
}

/// Limits and failures by every sign, and the sentence's shape: its words
/// capped, the bar's time dropped from the reason.
#[test]
fn an_error_result_is_a_limit_a_sign_in_or_a_failure_by_its_sign() {
    for (trouble, said) in [
        (Some("billing_error"), "x"),
        (None, "You have hit your usage limit"),
        (None, "Quota exceeded"),
        (None, "API Error: 429"),
        (None, "Rate Limit reached"),
    ] {
        let got = outcome_of(trouble, &json!({"is_error": true, "result": said}));
        assert!(
            matches!(got, Outcome::RateLimited { .. }),
            "{trouble:?} {said}: {got:?}"
        );
    }
    let Outcome::RateLimited { why, .. } = outcome_of(
        None,
        &json!({"is_error": true, "result": "usage limit reached|1790000000"}),
    ) else {
        panic!();
    };
    assert_eq!(why, "usage limit reached", "the time is not in the reason");

    let long = "z".repeat(500);
    let Outcome::Failed(why) = outcome_of(None, &json!({"is_error": true, "result": long})) else {
        panic!();
    };
    assert_eq!(why.chars().count(), 200);
    let Outcome::Failed(why) = outcome_of(Some("invalid_request"), &json!({"is_error": true}))
    else {
        panic!();
    };
    assert_eq!(why, "invalid_request", "trouble alone");
    // No `is_error`, but an error subtype: still a failure, never an answer.
    let Outcome::Failed(why) = outcome_of(None, &json!({"subtype": "error_max_turns"})) else {
        panic!();
    };
    assert_eq!(why, "the reply failed (error_max_turns)");
    let Outcome::Failed(why) = outcome_of(
        Some("authentication_failed"),
        &json!({"is_error": true, "result": "bad"}),
    ) else {
        panic!();
    };
    assert!(
        why.contains("not signed in (authentication_failed: bad)"),
        "{why}"
    );
}

/// An answer whose `structured_output` is no object is read from the text,
/// and one with neither is an answer with no value, to be asked again.
#[test]
fn a_structured_output_that_is_no_object_falls_back_to_the_text() {
    let Outcome::Answer { value, text, usage } = outcome_of(
        None,
        &json!({"subtype": "success", "result": "{\"ask\":\"q1\"}", "structured_output": [1]}),
    ) else {
        panic!();
    };
    assert_eq!(value, Some(json!({"ask": "q1"})));
    assert_eq!(text, "{\"ask\":\"q1\"}");
    assert_eq!(usage, None);
    let Outcome::Answer { value, .. } = outcome_of(None, &json!({"result": "no object here"}))
    else {
        panic!();
    };
    assert_eq!(value, None);
    let Outcome::Answer { usage, .. } = outcome_of(None, &json!({"result": "x", "usage": {}}))
    else {
        panic!();
    };
    assert_eq!(usage, Some(Usage::default()), "a usage that counts nothing");
}

/// Lines that are not a start, a reply or an assistant's are nothing, and
/// a start's lists may name entries as objects.
#[test]
fn other_lines_are_nothing_and_a_start_reads_named_objects() {
    let mut trouble = None;
    for line in [
        "",
        "[]",
        "42",
        r#"{"type":"system","subtype":"hook_started"}"#,
        r#"{"type":"stream_event"}"#,
        r#"{"subtype":"init"}"#,
    ] {
        assert_eq!(
            read_with(&Claude, line, &mut trouble),
            Event::Other,
            "{line}"
        );
    }
    let started = init(&json!({
        "tools": [{"name": "StructuredOutput"}],
        "mcp_servers": [{"name": "a"}, "b"],
        "slash_commands": [],
        "apiKeySource": 7,
    }));
    assert_eq!(started.tools, Some(vec!["StructuredOutput".to_string()]));
    assert_eq!(
        started.mcp_servers,
        Some(vec!["a".to_string(), "b".to_string()])
    );
    assert_eq!(started.key_source, None, "a number is no source");
    let not_a_list = init(&json!({"tools": "StructuredOutput"}));
    assert_eq!(not_a_list.tools, None, "a list it did not name");
}

/// Names a sentence shows are the first eight, the rest elided; the tools
/// are judged before the servers, the servers before the commands, and the
/// answer's tool is exactly `StructuredOutput`.
#[test]
fn the_lockdown_names_eight_and_judges_in_order() {
    let many: Vec<String> = (0..10).map(|n| format!("T{n}")).collect();
    let started = Started {
        tools: Some(many),
        mcp_servers: Some(vec!["srv".into()]),
        ..locked_down()
    };
    let why = Claude.lockdown_fault(&started).unwrap();
    assert!(why.contains("(T0, T1, T2, T3, T4, T5, T6, T7, …)"), "{why}");
    assert!(!why.contains("T8"), "{why}");
    assert!(!why.contains("srv"), "tools first: {why}");
    for lookalike in ["structuredoutput", "StructuredOutput2", "Structured Output"] {
        let started = Started {
            tools: Some(vec![lookalike.into()]),
            ..locked_down()
        };
        assert!(Claude.lockdown_fault(&started).is_some(), "{lookalike}");
    }
    let both = Started {
        mcp_servers: Some(vec!["srv".into()]),
        slash_commands: Some(vec!["cmd".into()]),
        ..locked_down()
    };
    let why = Claude.lockdown_fault(&both).unwrap();
    assert!(why.contains("MCP servers (srv)"), "{why}");
}

/// A key's source is a variable when it is NAMED like one; a login's
/// words are not.
#[test]
fn a_key_from_any_variable_is_refused_and_a_login_is_not() {
    let fault = |source: &str| {
        Claude.lockdown_fault(&Started {
            key_source: Some(source.into()),
            ..locked_down()
        })
    };
    for variable in [
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "CLAUDE_CODE_OAUTH_TOKEN",
        "X_1",
    ] {
        let why = fault(variable).unwrap_or_else(|| panic!("{variable}"));
        assert!(why.contains(&format!("a key from {variable}")), "{why}");
    }
    for login in [
        "none",
        "/login managed key",
        "claude.ai",
        "apiKeyHelper",
        "",
        "KEY",
    ] {
        assert_eq!(fault(login), None, "{login:?}");
    }
}

/// A start that does not say where its key comes from cannot show it is
/// not a variable's: fail closed, as for every other list it must name.
#[test]
fn a_start_that_names_no_key_source_is_refused() {
    let base = json!({"tools": ["StructuredOutput"], "mcp_servers": [], "slash_commands": []});
    let absent = Claude.lockdown_fault(&init(&base));
    assert!(absent.is_some(), "absent apiKeySource passed");
    for odd in [json!(null), json!(7), json!(["ANTHROPIC_API_KEY"])] {
        let mut fields = base.clone();
        fields["apiKeySource"] = odd.clone();
        assert!(
            Claude.lockdown_fault(&init(&fields)).is_some(),
            "{odd} passed"
        );
    }
}

/// A login check says yes only to a JSON object saying `loggedIn: true`.
#[test]
fn the_login_check_fails_closed() {
    assert!(Claude.probe_ok(b"  {\"loggedIn\":true,\"x\":1}\n", b""));
    for no in [
        &b""[..],
        b"null",
        b"[]",
        b"true",
        br#"{"loggedIn":"true"}"#,
        br#"{"loggedIn":1}"#,
        br#"{"loggedin":true}"#,
        br#"{"loggedIn":true} trailing"#,
        b"\xff\xfe",
    ] {
        assert!(!Claude.probe_ok(no, b""), "{}", String::from_utf8_lossy(no));
    }
    assert_eq!(
        Claude.probe_args().unwrap(),
        ["auth", "status", "--json"].map(OsString::from)
    );
}

/// A message is one line however many lines it holds, and the text comes
/// back whole.
#[test]
fn a_message_is_one_line_whatever_it_holds() {
    let text = "first\nsecond\r\n\"third\" \u{2028} end";
    let line = Claude.stdin_line(text);
    assert!(!line.contains('\n') && !line.contains('\r'), "{line:?}");
    let back: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(back["message"]["content"], text);
    assert_eq!(back["type"], "user");
}

/// With no model and no effort the arguments name neither, and the schema
/// closes them.
#[test]
fn claude_names_a_model_and_an_effort_only_when_it_has_them() {
    let mut settings = settings();
    settings.effort = None;
    let args: Vec<String> = Claude
        .args(&settings, None, "S", Path::new("/support"))
        .into_iter()
        .map(|a| a.into_string().unwrap())
        .collect();
    assert!(!args.iter().any(|a| a == "--model" || a == "--effort"));
    assert_eq!(args[args.len() - 2], "--json-schema");
    let at = args.iter().position(|a| a == "--system-prompt").unwrap();
    assert_eq!(args[at + 1], "S");
}

// ---------------------------------------------------------------------
// The environment and the program
// ---------------------------------------------------------------------

fn path_only(name: &str) -> Option<String> {
    (name == "PATH").then(|| "/usr/bin".to_string())
}

fn launch_with(env: &dyn Fn(&str) -> Option<String>) -> Result<Launch, String> {
    let tool = std::env::current_exe().unwrap();
    Launch::new(&settings(), Some(tool.to_str().unwrap()), env)
}

/// Claude Code's own variable is fixed, never the parent's, and the whole
/// environment is the allowlist alone.
#[test]
fn the_process_never_updates_itself_and_sees_only_the_allowlist() {
    assert_eq!(Claude.fixed_env(), [("DISABLE_AUTOUPDATER", "1")]);
    let parent = |name: &str| match name {
        "DISABLE_AUTOUPDATER" => Some("0".to_string()),
        "ANTHROPIC_API_KEY" => Some("fine-looking".to_string()),
        "EDITOR" => Some("vim".to_string()),
        "HOME" => Some("/home/tester".to_string()),
        "USER" => Some(String::new()),
        "CLAUDE_CONFIG_DIR" => Some("/home/tester/.claude".to_string()),
        _ => None,
    };
    let launch = launch_with(&parent).unwrap();
    let tmp = Path::new("/session/tmp");
    let env = launch.env(tmp, tmp).unwrap();
    let get = |name: &str| {
        env.iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.to_string_lossy().into_owned())
    };
    assert_eq!(get("DISABLE_AUTOUPDATER").as_deref(), Some("1"));
    assert_eq!(get("HOME").as_deref(), Some("/home/tester"));
    assert_eq!(
        get("CLAUDE_CONFIG_DIR").as_deref(),
        Some("/home/tester/.claude")
    );
    assert_eq!(get("TMPDIR").as_deref(), Some("/session/tmp"));
    assert_eq!(get("TERM").as_deref(), Some("dumb"));
    assert_eq!(get("NO_COLOR").as_deref(), Some("1"));
    assert_eq!(get("USER"), None, "an empty value is not passed");
    assert_eq!(get("ANTHROPIC_API_KEY"), None);
    assert_eq!(get("EDITOR"), None);
    let mut names: Vec<&str> = env.iter().map(|(n, _)| n.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), env.len(), "no name twice");
    let debug = format!("{launch:?}");
    assert!(!debug.contains("/home/tester"), "values stay out: {debug}");
    assert!(debug.contains("HOME"), "{debug}");
}

/// A name no CLI is given, or a key-shaped value, refuses the environment
/// by name, never showing the value; a tool this build does not speak is
/// refused before anything runs.
#[test]
fn the_environment_refuses_a_forbidden_name_or_value_without_showing_it() {
    let mut launch = launch_with(&path_only).unwrap();
    launch.passed.push(("ANTHROPIC_API_KEY", "harmless".into()));
    let why = launch.env(Path::new("/t"), Path::new("/t")).unwrap_err();
    assert!(why.contains("ANTHROPIC_API_KEY is never given"), "{why}");
    let mut launch = launch_with(&path_only).unwrap();
    launch
        .passed
        .push(("USER", "Bearer SECRETVALUE0123456789".into()));
    let why = launch.env(Path::new("/t"), Path::new("/t")).unwrap_err();
    assert!(why.contains("USER looks like a key"), "{why}");
    assert!(!why.contains("SECRETVALUE"), "{why}");
    // A key-shaped TMPDIR is refused as well.
    let launch = launch_with(&path_only).unwrap();
    assert!(
        launch
            .env(Path::new("/sk-0123456789abcdefghij"), Path::new("/t"))
            .is_err()
    );

    let mut unsupported = settings();
    unsupported.model = "cursor-agent:pro".into();
    let why = Launch::new(&unsupported, Some("/bin/sh"), &path_only).unwrap_err();
    assert!(why.contains("not a CLI this build plays"), "{why}");
}

/// The names that are never given, whatever their case.
#[test]
fn forbidden_names_are_matched_by_prefix_suffix_and_word() {
    for name in [
        "anthropic_whatever",
        "Gh_Host",
        "GOOGLE_APPLICATION_CREDENTIALS",
        "foo_api_key",
        "FOO_TOKEN",
        "my_secret_thing",
        "DB_PASSWORD",
        "database_url",
        "ssh_auth_sock",
        "BAYLEE_X",
    ] {
        assert!(forbidden(name), "{name}");
    }
    for name in [
        "PATH",
        "HOME",
        "XDG_CONFIG_HOME",
        "TOKENIZERS",
        "GHOST",
        "AWSOME",
        "LANG",
    ] {
        assert!(!forbidden(name), "{name}");
    }
}

/// Every marker, at its length, at a word's start.
#[test]
fn a_key_is_a_marker_at_a_words_start_with_enough_key_characters() {
    let run = |n: usize| "a".repeat(n);
    assert!(shaped_like_a_key(&format!("sk-{}", run(16))));
    assert!(!shaped_like_a_key(&format!("sk-{}", run(15))), "one short");
    for prefix in ["ghp_", "gho_", "ghu_", "ghs_", "ghr_", "github_pat_"] {
        assert!(
            shaped_like_a_key(&format!("{prefix}{}", run(20))),
            "{prefix}"
        );
        assert!(
            !shaped_like_a_key(&format!("{prefix}{}", run(19))),
            "{prefix} short"
        );
        assert!(
            shaped_like_a_key(&format!("a={prefix}{}", run(20))),
            "{prefix} after ="
        );
    }
    assert!(shaped_like_a_key("Authorization: Bearer x"));
    assert!(shaped_like_a_key("bearer x"));
    assert!(!shaped_like_a_key("Bearer "), "nothing after the space");
    assert!(!shaped_like_a_key("Bearer"), "no space");
    assert!(shaped_like_a_key("x-api-key"), "the header's name alone");
    assert!(shaped_like_a_key("curl -H x-api-key:abc"));
    // Inside a word is no start; after a path or space is.
    assert!(!shaped_like_a_key(&format!("task-{}", run(20))));
    // A provider's long marker counts glued to a word too: twenty key
    // characters after `ghp_` are a token wherever they stand.
    assert!(shaped_like_a_key(&format!("my_ghp_{}", run(20))));
    assert!(shaped_like_a_key(&format!("/x/sk-{}", run(16))));
    assert!(
        shaped_like_a_key(&format!("é sk-{}", run(16))),
        "after a non-ASCII"
    );
    // One embedded marker does not hide a second, anchored one.
    assert!(shaped_like_a_key(&format!(
        "desk-{} sk-{}",
        run(20),
        run(16)
    )));
    assert!(!shaped_like_a_key(""));
    assert!(!shaped_like_a_key("sk-"));
}

/// The seat's and the client's reading of what is key-shaped agree: a
/// value one calls a key and the other does not is either a leak (the
/// seat lets through what the client refuses) or a refusal for nothing.
/// One definition, client-core's, and the seat's start refuses a passed
/// value exactly where it says key.
#[test]
fn the_seat_and_the_client_agree_on_what_a_key_looks_like() {
    let run = "a".repeat(24);
    for (value, key) in [
        (format!("sk-ant-api03-{run}"), true),
        (format!("/Users/sk-ant-{run}"), true),
        (format!("ghp_{run}"), true),
        (format!("github_pat_{run}"), true),
        ("Bearer abc".to_string(), true),
        ("/opt/desk-tools-collection/bin".to_string(), false),
        ("/usr/local/bin:/usr/bin".to_string(), false),
        (String::new(), false),
    ] {
        assert_eq!(shaped_like_a_key(&value), key, "client-core: {value}");
        let env = |name: &str| match name {
            "HOME" => Some(value.clone()),
            "PATH" => Some("/nowhere".to_string()),
            _ => None,
        };
        let refused = Launch::new(&settings(), Some("/bin/sh"), &env)
            .err()
            .is_some_and(|why| why.contains("HOME looks like a key"));
        assert_eq!(refused, key, "seat: {value}");
    }
}

/// A command is a file this user may run: not a directory, not a file
/// without its execute bit.
#[cfg(unix)]
#[test]
fn a_command_is_a_file_that_runs() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("baylee-cli-runnable-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let plain = dir.join("claude");
    std::fs::write(&plain, "#!/bin/sh\n").unwrap();
    std::fs::set_permissions(&plain, std::fs::Permissions::from_mode(0o644)).unwrap();
    let none = |_: &str| None;
    let why = program(CliTool::Claude, Some(plain.to_str().unwrap()), &none).unwrap_err();
    assert!(why.contains("not a program"), "{why}");
    let why = program(CliTool::Claude, Some(dir.to_str().unwrap()), &none).unwrap_err();
    assert!(why.contains("not a program"), "{why}");
    let on_path = |name: &str| (name == "PATH").then(|| dir.display().to_string());
    assert!(
        program(CliTool::Claude, None, &on_path).is_err(),
        "not runnable"
    );
    std::fs::set_permissions(&plain, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(program(CliTool::Claude, None, &on_path), Ok(plain.clone()));
    assert_eq!(
        program(CliTool::Claude, None, &|_| None),
        Err("claude is not on PATH: install it, or name its program in a profile's command".into())
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A session's directory is this user's alone, holds an empty `work` and a
/// `tmp`, takes only safe characters of the game's name, and is gone when
/// dropped.
#[cfg(unix)]
#[test]
fn a_session_directory_is_private_and_removed_with_it() {
    use std::os::unix::fs::PermissionsExt;
    let dir = SessionDir::new("../../etc/pass wd-1", 3).unwrap();
    let root = dir.root.clone();
    let name = root.file_name().unwrap().to_str().unwrap().to_string();
    assert_eq!(root.parent().unwrap(), std::env::temp_dir(), "{root:?}");
    assert!(name.starts_with("baylee-cli-etcpasswd-1-3-"), "{name}");
    for part in [&root, &dir.work(), &dir.tmp()] {
        let mode = std::fs::metadata(part).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "{part:?}");
    }
    assert_eq!(std::fs::read_dir(dir.work()).unwrap().count(), 0);
    let (a, b) = (
        SessionDir::new("x", 0).unwrap(),
        SessionDir::new("x", 0).unwrap(),
    );
    assert_ne!(a.root, b.root, "unique");
    drop(dir);
    assert!(!root.exists());
    let long = SessionDir::new(&"g".repeat(100), 0).unwrap();
    let tag = long.root.file_name().unwrap().to_str().unwrap().to_string();
    assert!(
        tag.starts_with(&format!("baylee-cli-{}-0-", "g".repeat(24))),
        "{tag}"
    );
}

// ---------------------------------------------------------------------
// Cooldown and lockout
// ---------------------------------------------------------------------

fn mind() -> CliMind {
    let launch = launch_with(&path_only).unwrap();
    CliMind::new(settings(), launch, Limits::default())
}

/// With no time named, each limit cools for twice the last, to fifteen
/// minutes, and an answer starts it over.
#[test]
fn the_cooldown_doubles_to_fifteen_minutes_and_an_answer_resets_it() {
    let mind = mind();
    assert_eq!(mind.cooling(), None);
    let step = |mind: &CliMind| {
        mind.cool(None);
        mind.cooling().unwrap()
    };
    let first = step(&mind);
    assert!(first <= Duration::from_secs(60) && first > Duration::from_secs(55));
    let second = step(&mind);
    assert!(second <= Duration::from_secs(120) && second > Duration::from_secs(115));
    for _ in 0..6 {
        step(&mind);
    }
    assert_eq!(lock(&mind.cooldown).next, MAX_COOLDOWN, "capped");
    mind.cooled();
    assert_eq!(mind.cooling(), None);
    assert_eq!(lock(&mind.cooldown).next, Duration::from_secs(60));
    // A believed time does not double it.
    mind.cool(Some(Duration::from_secs(30)));
    assert_eq!(lock(&mind.cooldown).next, Duration::from_secs(60));
}

/// The first reason a mind is locked out for stays; later ones add
/// nothing; and a lockout with no seats still records.
#[test]
fn the_first_lockout_reason_stays() {
    let out = Mutex::new(None);
    lock_out(&out, None, "first");
    lock_out(&out, None, "second");
    assert_eq!(lock(&out).as_deref(), Some("first"));
    let seats: Seats = Mutex::new(BTreeMap::new());
    let empty = Mutex::new(None);
    lock_out(&empty, Some(&seats), "again");
    assert_eq!(lock(&empty).as_deref(), Some("again"));
}

/// The Agy dialect: arguments, stdin format, event parsing, and probes.
#[test]
fn agy_dialect_args_and_events() {
    use super::agy::Agy;

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

    // Stdin lines: first line includes system prompt, subsequent does not.
    let line1 = Agy.stdin_line("THE GAME\nYou are P1 at a table");
    let val1: Value = serde_json::from_str(&line1).unwrap();
    assert_eq!(val1["event"], "user");
    assert!(
        val1["message"]["content"]
            .as_str()
            .unwrap()
            .contains("You are playing a game of Magic")
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
