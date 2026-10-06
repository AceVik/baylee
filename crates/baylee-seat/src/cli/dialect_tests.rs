//! Each dialect without a process: the arguments it starts its tool with
//! (golden), the lines it reads (fixtures written from each tool's
//! documentation or source, `docs/llm-seat.md` §"A CLI as the model"),
//! and what every dialect holds to: no flag that approves anything, no
//! key in an argument, and one answer to whether a process keeps the
//! conversation, shared with the settings panel.

use super::agy::Agy;
use super::claude::Claude;
use super::codex::{Codex, DISABLED_FEATURES};
use super::dialect::{Dialect, Event, Outcome, Started, Wire};
use super::junie::Junie;
use super::opencode::Opencode;
use super::*;
use crate::llm::Spec;
use baylee_client_core::llmseat::clis;

fn settings_for(model: &str, effort: Option<&str>) -> Settings {
    let mut settings = Settings::new(&Spec::parse(&format!("cli:{model}")).unwrap().unwrap());
    settings.effort = effort.map(str::to_string);
    settings
}

fn args_of(dialect: &dyn Dialect, model: Option<&str>, effort: Option<&str>) -> Vec<String> {
    let name = dialect.tool().name();
    let settings = settings_for(
        &model.map_or(name.to_string(), |m| format!("{name}:{m}")),
        effort,
    );
    dialect
        .args(&settings, model, "SYSTEM", Path::new("/s"))
        .into_iter()
        .map(|arg| arg.into_string().unwrap())
        .collect()
}

/// Reads `lines` in order on one wire, as a process's reader does: a line
/// the dialect asks to read again is read twice.
fn read_all(dialect: &dyn Dialect, lines: &[&str]) -> Vec<Event> {
    let mut wire = Wire::default();
    let mut events = Vec::new();
    for line in lines {
        events.push(dialect.read_event(line, &mut wire));
        if std::mem::take(&mut wire.again) {
            events.push(dialect.read_event(line, &mut wire));
        }
    }
    events
}

fn the_dialects() -> Vec<Box<dyn Dialect>> {
    vec![
        Box::new(Claude),
        Box::new(Agy),
        Box::new(Codex),
        Box::new(Opencode),
        Box::new(Junie),
    ]
}

// ---------------------------------------------------------------------
// What every dialect holds to
// ---------------------------------------------------------------------

/// No dialect starts its tool with a flag that approves what the model
/// does, or with a credential in its arguments: whatever the settings, the
/// tool's own approval (or none) holds, and its login is its own.
#[test]
fn no_dialect_approves_anything_or_passes_a_credential() {
    let approving = [
        "--dangerously-skip-permissions",
        "--allow-dangerously-skip-permissions",
        "--dangerously-bypass-approvals-and-sandbox",
        "--dangerously-bypass-hook-trust",
        "--approve-for-me",
        "--auto",
        "--yolo",
        "--brave",
        "--force",
        "--trust",
        "--allow-all-tools",
        "--full-auto",
        "acceptEdits",
        "bypassPermissions",
        "danger-full-access",
        "accept-edits",
    ];
    for dialect in the_dialects() {
        let name = dialect.tool().name();
        assert_eq!(dialect.tool(), CliTool::named(name).unwrap());
        for args in [
            args_of(dialect.as_ref(), None, None),
            args_of(dialect.as_ref(), Some("m"), Some("high")),
        ] {
            for arg in &args {
                let flag = arg.split('=').next().unwrap_or_default();
                assert!(!approving.contains(&flag), "{name}: {arg}");
                let lower = arg.to_lowercase();
                for word in [
                    "dangerous",
                    "yolo",
                    "bypass",
                    "api-key",
                    "api_key",
                    "--auth",
                ] {
                    assert!(
                        !lower.starts_with(word) && !lower.starts_with(&format!("--{word}")),
                        "{name}: {arg}"
                    );
                }
                assert!(!lower.contains("dangerously"), "{name}: {arg}");
                assert!(!shaped_like_a_key(arg) || arg == "SYSTEM", "{name}: {arg}");
            }
        }
    }
}

/// Every tool this build speaks has a dialect, and the settings panel's
/// word on whether a process keeps the conversation is the dialect's.
#[test]
fn every_tool_has_a_dialect_that_agrees_with_the_panel() {
    for tool in CliTool::ALL {
        let dialect = dialect(tool);
        assert_eq!(dialect.tool(), tool);
        assert_eq!(
            dialect.one_shot(),
            !clis::choices(tool).keeps_conversation,
            "{tool:?}"
        );
    }
}

/// Each tool's own variables pass where the parent has them, its fixed
/// and session variables are set whatever the parent has, and none of them
/// is a name no CLI is given.
#[test]
fn each_tools_environment_is_its_login_and_its_own_files() {
    for dialect in the_dialects() {
        for name in dialect.passed_env() {
            assert!(!forbidden(name), "{name}");
        }
        for (name, value) in dialect.fixed_env() {
            assert!(!forbidden(name), "{name}");
            assert!(!shaped_like_a_key(value), "{name}");
        }
        for (name, value) in dialect.session_env(Path::new("/s/support")) {
            assert!(!forbidden(name), "{name}");
            assert!(
                Path::new(&value).starts_with("/s/support"),
                "{name} names the session's own: {value:?}"
            );
        }
    }
    let tool = std::env::current_exe().unwrap();
    let parent = |name: &str| match name {
        "HOME" => Some("/home/tester".to_string()),
        "XDG_DATA_HOME" => Some("/home/tester/.data".to_string()),
        "XDG_CONFIG_HOME" => Some("/home/tester/.config".to_string()),
        "OPENCODE_CONFIG" => Some("/home/tester/theirs.json".to_string()),
        "CODEX_HOME" => Some("/home/tester/.codex-seat".to_string()),
        "CODEX_API_KEY" | "JUNIE_API_KEY" => Some("whatever".to_string()),
        _ => None,
    };
    let env_of = |model: &str| {
        let launch = Launch::new(
            &settings_for(model, None),
            Some(tool.to_str().unwrap()),
            &parent,
        )
        .unwrap();
        let env = launch
            .env(Path::new("/s/tmp"), Path::new("/s/support"))
            .unwrap();
        env.into_iter()
            .map(|(name, value)| (name, value.to_string_lossy().into_owned()))
            .collect::<BTreeMap<_, _>>()
    };
    let opencode = env_of("opencode");
    assert_eq!(opencode["XDG_DATA_HOME"], "/home/tester/.data", "its login");
    assert_eq!(
        opencode["XDG_CONFIG_HOME"], "/s/support/config",
        "not theirs"
    );
    assert_eq!(opencode["OPENCODE_CONFIG"], "/s/support/opencode.json");
    assert_eq!(opencode["OPENCODE_DB"], ":memory:");
    assert_eq!(opencode["OPENCODE_DISABLE_PROJECT_CONFIG"], "1");
    let codex = env_of("codex");
    assert_eq!(codex["CODEX_HOME"], "/home/tester/.codex-seat");
    assert!(!codex.contains_key("CODEX_API_KEY"));
    assert!(!codex.contains_key("XDG_DATA_HOME"));
    let junie = env_of("junie");
    assert!(!junie.contains_key("JUNIE_API_KEY"));
    for env in [&opencode, &codex, &junie] {
        let mut names: Vec<&String> = env.keys().collect();
        names.dedup();
        assert_eq!(names.len(), env.len());
    }
}

// ---------------------------------------------------------------------
// Codex
// ---------------------------------------------------------------------

/// `codex exec`, locked down: user config, rules and project docs off,
/// our instructions and schema by file, every tool feature off, read-only,
/// nothing kept, the prompt from stdin; model and effort where named.
#[test]
fn codex_runs_exec_with_our_instructions_and_no_tools() {
    let args = args_of(&Codex, Some("gpt-6.1-sol"), Some("low"));
    let mut expected: Vec<String> = [
        "exec",
        "--json",
        "--color",
        "never",
        "--ephemeral",
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
    assert_eq!(
        serde_json::from_str::<Value>(&files[1].1).unwrap(),
        prompt::answer_schema()
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
    assert_eq!(events[1..4], [Event::Other, Event::Other, Event::Other]);
    let Event::Reply(Outcome::Answer { value, text, usage }) = &events[4] else {
        panic!("{:?}", events[4]);
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
            let Event::Breach(why) = &read[0] else {
                panic!("{kind}: {read:?}");
            };
            assert!(why.contains(kind), "{why}");
        }
    }
    let warning = r#"{"type":"item.completed","item":{"id":"i","type":"error","message":"x"}}"#;
    assert_eq!(read_all(&Codex, &[warning]), [Event::Other]);
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
    assert_eq!(limited[1], Event::Other, "a retry ends nothing");
    assert!(matches!(
        &limited[2],
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
        failed[1],
        Event::Reply(Outcome::Failed("stream disconnected".into()))
    );
    assert_eq!(
        failed[2],
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
        retries[0],
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

// ---------------------------------------------------------------------
// opencode
// ---------------------------------------------------------------------

/// `opencode run`: no external plugin, JSON events, the session's agent,
/// a title so none is written by a model; model and variant where named;
/// never a message in argv.
#[test]
fn opencode_runs_its_seat_agent_with_a_title_and_no_message_in_argv() {
    assert_eq!(
        args_of(&Opencode, Some("anthropic/claude-sonnet-5"), Some("max")),
        [
            "run",
            "--pure",
            "--format",
            "json",
            "--agent",
            "seat",
            "--title",
            "seat",
            "--model",
            "anthropic/claude-sonnet-5",
            "--variant",
            "max",
        ]
    );
    assert_eq!(args_of(&Opencode, None, None).len(), 8);
    assert!(Opencode.one_shot());
    assert_eq!(Opencode.stdin_line("q1 \"x\""), "q1 \"x\"");
}

/// The configuration it is pointed at denies every tool, for the agent
/// too, connects no MCP server, reads no instruction file, shares and
/// snapshots nothing, and makes our instructions the agent's prompt.
#[test]
fn opencodes_configuration_denies_every_tool_and_carries_ours() {
    let files = Opencode.files(&settings_for("opencode", None), "SYSTEM \"quoted\"");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, "opencode.json");
    let config: Value = serde_json::from_str(&files[0].1).unwrap();
    assert_eq!(config["permission"], json!({"*": "deny"}));
    assert_eq!(config["agent"]["seat"]["permission"], json!({"*": "deny"}));
    assert_eq!(config["agent"]["seat"]["prompt"], "SYSTEM \"quoted\"");
    assert_eq!(config["agent"]["seat"]["mode"], "primary");
    assert_eq!(config["mcp"], json!({}));
    assert_eq!(config["instructions"], json!([]));
    assert_eq!(config["share"], "disabled");
    assert_eq!(config["autoupdate"], false);
    assert_eq!(config["snapshot"], false);
    let fixed: BTreeMap<_, _> = Opencode.fixed_env().iter().copied().collect();
    for name in [
        "OPENCODE_DISABLE_PROJECT_CONFIG",
        "OPENCODE_DISABLE_CLAUDE_CODE",
        "OPENCODE_DISABLE_EXTERNAL_SKILLS",
        "OPENCODE_DISABLE_AUTOUPDATE",
    ] {
        assert_eq!(fixed[name], "1", "{name}");
    }
    assert_eq!(fixed["OPENCODE_DB"], ":memory:");
}

/// opencode says nothing at its start: its first line is the start and is
/// read for itself as well; text parts make the answer, the step's finish
/// ends it with that step's tokens (reasoning is output, the cache apart).
#[test]
fn opencode_starts_on_its_first_line_and_answers_at_the_steps_finish() {
    let events = read_all(
        &Opencode,
        &[
            r#"{"type":"step_start","timestamp":1,"sessionID":"ses_1","part":{"id":"p1","type":"step-start"}}"#,
            r#"{"type":"text","timestamp":2,"sessionID":"ses_1","part":{"type":"text","text":"{\"ask\":\"q1\",","time":{"start":1,"end":2}}}"#,
            r#"{"type":"text","timestamp":3,"sessionID":"ses_1","part":{"type":"text","text":"\"pick\":[\"a\"]}","time":{"start":2,"end":3}}}"#,
            r#"{"type":"step_finish","timestamp":4,"sessionID":"ses_1","part":{"type":"step-finish","reason":"stop","cost":0,"tokens":{"input":812,"output":40,"reasoning":10,"cache":{"read":3000,"write":200}}}}"#,
        ],
    );
    assert_eq!(events[0], Event::Started(Started::default()));
    assert_eq!(events[1..4], [Event::Other, Event::Other, Event::Other]);
    let Event::Reply(Outcome::Answer { value, usage, .. }) = &events[4] else {
        panic!("{events:?}");
    };
    assert_eq!(value.as_ref().unwrap()["pick"], json!(["a"]));
    let usage = usage.unwrap();
    assert_eq!(
        (
            usage.input,
            usage.output,
            usage.cache_read,
            usage.cache_write
        ),
        (812, 50, 3000, 200)
    );
    assert!(!Opencode.usage_is_cumulative());
    // Not JSON, or no type: nothing, and no start either.
    assert_eq!(
        read_all(&Opencode, &["warming up", "{}"]),
        [Event::Other, Event::Other]
    );
}

/// A tool used is a breach; an error is a limit by its status or words, a
/// sign-in missing, or a failure, and an error as the first line is still
/// read after the start it makes.
#[test]
fn opencode_reads_a_tool_as_a_breach_and_an_error_by_its_kind() {
    let start = r#"{"type":"step_start","sessionID":"s","part":{}}"#;
    let tool = r#"{"type":"tool_use","sessionID":"s","part":{"type":"tool","tool":"bash","state":{"status":"completed"}}}"#;
    let read = read_all(&Opencode, &[start, tool]);
    let Event::Breach(why) = &read[2] else {
        panic!("{read:?}");
    };
    assert!(why.contains("bash"), "{why}");
    let limited = r#"{"type":"error","sessionID":"s","error":{"name":"APIError","data":{"message":"Too Many Requests","statusCode":429,"isRetryable":true}}}"#;
    let read = read_all(&Opencode, &[limited]);
    assert_eq!(read[0], Event::Started(Started::default()));
    assert!(matches!(read[1], Event::Reply(Outcome::RateLimited { .. })));
    let auth = r#"{"type":"error","sessionID":"s","error":{"name":"ProviderAuthError","data":{"providerID":"anthropic","message":"no credentials"}}}"#;
    let Event::Reply(Outcome::Failed(why)) = &read_all(&Opencode, &[start, auth])[2] else {
        panic!();
    };
    assert!(why.contains("not signed in"), "{why}");
    let other =
        r#"{"type":"error","sessionID":"s","error":{"name":"ContextOverflowError","data":{}}}"#;
    assert_eq!(
        read_all(&Opencode, &[start, other])[2],
        Event::Reply(Outcome::Failed("ContextOverflowError".into()))
    );
}

/// Signed in where `opencode auth list` names a sign-in; a stored key is
/// not one.
#[test]
fn opencode_is_signed_in_by_an_oauth_credential() {
    let listed =
        b"Credentials ~/.local/share/opencode/auth.json\n  Anthropic oauth\n1 credentials\n";
    assert!(Opencode.probe_ok(listed, b""));
    assert!(!Opencode.probe_ok(b"  OpenAI api\n1 credentials\n", b""));
    assert!(!Opencode.probe_ok(b"0 credentials\n", b""));
}

// ---------------------------------------------------------------------
// Junie
// ---------------------------------------------------------------------

/// Junie non-interactive: a JSON task in, a JSON stream out, every
/// default location off, the session's own empty guidelines, extensions
/// and caches, chat mode, ours added to its prompt; model and effort
/// where named.
#[test]
fn junie_runs_with_every_default_location_off() {
    assert_eq!(
        args_of(&Junie, Some("sonnet"), Some("high")),
        [
            "--input-format=json",
            "--output-format=json-stream",
            "--skip-update-check",
            "--share-anonymous-statistics=false",
            "--config-default-locations=false",
            "--mcp-default-locations=false",
            "--skill-default-locations=false",
            "--command-default-location=false",
            "--agent-default-location=false",
            "--model-default-locations=false",
            "--agent-mode=chat",
            "--extensions-default-location=/s/extensions",
            "--guidelines-filename=/s/guidelines.md",
            "--cache-dir=/s/cache",
            "--system-prompt=SYSTEM",
            "--model=sonnet",
            "--effort=high",
        ]
    );
    assert!(
        !args_of(&Junie, None, None)
            .iter()
            .any(|a| a.starts_with("--model=") || a.starts_with("--effort="))
    );
    assert_eq!(
        Junie.files(&settings_for("junie", None), "S"),
        [("guidelines.md", String::new())]
    );
    assert!(Junie.one_shot());
    let task: Value = serde_json::from_str(&Junie.stdin_line("q1\n\"x\"")).unwrap();
    assert_eq!(task, json!({"task": "q1\n\"x\""}));
}

/// Junie's stream as its makers' own fixture shows it: its banner is
/// nothing, its session is the start, the task's result step is allowed,
/// and the result carries the answer and the whole task's tokens, summed
/// over its models, found by their shape.
#[test]
fn junie_reads_its_result_and_its_per_model_usage() {
    let events = read_all(
        &Junie,
        &[
            "Junie 26.9.22",
            r#"{"type":"session","timestamp":1,"sessionId":"session-test"}"#,
            r#"{"type":"step","timestamp":4,"name":"TASK RESULT","details":"{\"ask\":\"q1\"}"}"#,
            r#"{"type":"result","timestamp":5,"result":"{\"ask\":\"q1\",\"pick\":[\"a\"]}","changes":[],"errorCode":[{"model":"claude-sonnet-5","calls":2,"cost":0.05,"inputTokens":4,"cacheInputTokens":100,"cacheCreateTokens":200,"outputTokens":20},{"model":"small","inputTokens":6,"outputTokens":1}]}"#,
        ],
    );
    assert_eq!(events[0], Event::Other);
    assert_eq!(events[1], Event::Started(Started::default()));
    assert_eq!(events[2], Event::Other);
    let Event::Reply(Outcome::Answer { value, usage, .. }) = &events[3] else {
        panic!("{events:?}");
    };
    assert_eq!(value.as_ref().unwrap()["ask"], "q1");
    let usage = usage.unwrap();
    assert_eq!(
        (
            usage.input,
            usage.output,
            usage.cache_read,
            usage.cache_write
        ),
        (10, 21, 100, 200)
    );
    // Under another key, found the same.
    let renamed = read_all(
        &Junie,
        &[r#"{"type":"result","result":"x","usage":[{"inputTokens":7,"outputTokens":2}]}"#],
    );
    let Event::Reply(Outcome::Answer { usage, .. }) = &renamed[0] else {
        panic!();
    };
    assert_eq!(usage.unwrap().input, 7);
    assert!(Junie.usage_is_cumulative());
}

/// A step of its own is a tool used; errors on the result, or a line
/// before it, are a spent balance or limit, or a failure.
#[test]
fn junie_reads_a_step_as_a_breach_and_its_errors_by_their_words() {
    let step = r#"{"type":"step","timestamp":2,"name":"Opened file","details":"ping.txt"}"#;
    let Event::Breach(why) = &read_all(&Junie, &[step])[0] else {
        panic!();
    };
    assert!(why.contains("Opened file"), "{why}");
    let spent = r#"{"type":"result","result":"","errors":["Junie: Insufficient Account Balance. All tokens on your balance are spent."]}"#;
    assert!(matches!(
        read_all(&Junie, &[spent])[0],
        Event::Reply(Outcome::RateLimited { .. })
    ));
    let events = read_all(
        &Junie,
        &[
            r#"{"type":"error","message":"Cannot find authorization"}"#,
            r#"{"type":"result","result":""}"#,
        ],
    );
    assert_eq!(
        events[1],
        Event::Reply(Outcome::Failed("Cannot find authorization".into()))
    );
    assert!(Junie.probe_ok(b"Junie version: 26.9.22 (3419.29)\n", b""));
    assert!(!Junie.probe_ok(b"", b""));
}

// ---------------------------------------------------------------------
// agy
// ---------------------------------------------------------------------

/// agy's start must name its tools and connect no MCP server; a step of
/// the tool kind is a breach, a response step is nothing.
#[test]
fn agy_names_its_tools_and_uses_none() {
    assert!(Agy.lockdown_fault(&Started::default()).is_some());
    let named = Started {
        tools: Some(vec!["run_command".into()]),
        ..Started::default()
    };
    assert_eq!(Agy.lockdown_fault(&named), None);
    let served = Started {
        mcp_servers: Some(vec!["github".into()]),
        ..named.clone()
    };
    assert!(Agy.lockdown_fault(&served).unwrap().contains("github"));
    let init = r#"{"event":"init","conversation_id":"c3b6","init":{"cwd":"/w","tools":["ask_permission","run_command","write_to_file"],"permission_mode":"request-review"}}"#;
    let Event::Started(started) = &read_all(&Agy, &[init])[0] else {
        panic!();
    };
    assert_eq!(Agy.lockdown_fault(started), None);
    let tool = r#"{"event":"step_update","step_update":{"conversation_id":"c","step_index":2,"state":"DONE","step_type":"tool","tool_info":{"name":"run_command","parameters":{}}}}"#;
    let Event::Breach(why) = &read_all(&Agy, &[tool])[0] else {
        panic!();
    };
    assert!(why.contains("run_command"), "{why}");
    let response = r#"{"event":"step_update","step_update":{"conversation_id":"c","step_index":3,"state":"DONE","step_type":"agent_response","text_delta":"x","usage":{"input_tokens":1}}}"#;
    assert_eq!(read_all(&Agy, &[response]), [Event::Other]);
}
