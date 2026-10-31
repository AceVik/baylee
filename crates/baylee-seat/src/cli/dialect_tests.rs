//! Each dialect without a process: the arguments it starts its tool with
//! (golden), the lines it reads (fixtures written from each tool's
//! documentation or source, `docs/llm-seat.md` §"A CLI as the model"),
//! and what every dialect holds to: no flag that approves anything, no
//! key in an argument, and one answer to whether a process keeps the
//! conversation, shared with the settings panel.

use super::agy::Agy;
use super::claude::Claude;
use super::codex::Codex;
#[cfg(unix)]
use super::codex::DISABLED_FEATURES;
use super::dialect::{Dialect, Event, Outcome, Started, Wire};
use super::junie::Junie;
use super::opencode::Opencode;
use super::*;
use crate::llm::Spec;
#[cfg(unix)]
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
/// word on whether the conversation goes on across turns is the
/// dialect's: a process that holds it, or a one-shot tool that resumes it.
/// One that resumes keeps its sessions either in the seat's store or in a
/// place of its own it names, never neither; one that does not leaves its
/// arguments as they are, keeps nothing in a store, and keeps its sessions
/// beside its login only while one process holds them (Junie's ACP), from
/// where the seat removes its own.
// Unix only: unix paths and a home found by HOME, as the CLI seat has
// them (Windows sets no HOME and joins with \; not yet supported there).
#[cfg(unix)]
#[test]
fn every_tool_has_a_dialect_that_agrees_with_the_panel() {
    let home = |name: &str| (name == "HOME").then(|| OsString::from("/home/t"));
    for tool in CliTool::ALL {
        let dialect = dialect(tool);
        assert_eq!(dialect.tool(), tool);
        assert_eq!(
            !dialect.one_shot() || dialect.resumes(),
            clis::choices(tool).keeps_conversation,
            "{tool:?}"
        );
        let in_store = !dialect.store_env(Path::new("/s")).is_empty();
        let outside = dialect.sessions_root(&home);
        if dialect.resumes() {
            assert!(dialect.one_shot(), "{tool:?}");
            assert_ne!(in_store, outside.is_some(), "{tool:?}");
            if let Some(root) = outside {
                assert!(root.starts_with("/home/t"), "{tool:?}: {root:?}");
            }
            if dialect.usage_spans_resumes() {
                assert!(dialect.usage_is_cumulative(), "{tool:?}");
            }
        } else {
            let mut args = vec![OsString::from("a")];
            dialect.resume(&mut args, "ses_1");
            assert_eq!(args, [OsString::from("a")], "{tool:?}");
            assert!(!in_store, "{tool:?}");
            if let Some(root) = outside {
                assert!(!dialect.one_shot(), "{tool:?}: kept, never resumed");
                assert!(root.starts_with("/home/t"), "{tool:?}: {root:?}");
            }
            assert!(!dialect.usage_spans_resumes(), "{tool:?}");
        }
    }
}

/// A conversation's id is handed back in an argument only as letters,
/// digits, `_` and `-`, never first, and not too long.
#[test]
fn a_conversation_id_cannot_make_an_argument_of_its_own() {
    use super::dialect::conversation_id;
    assert_eq!(conversation_id("ses_1a-B").as_deref(), Some("ses_1a-B"));
    for bad in [
        "",
        "-c",
        "--continue",
        "a b",
        "a=b",
        "ses/1",
        "ü",
        &"x".repeat(129),
    ] {
        assert_eq!(conversation_id(bad), None, "{bad:?}");
    }
}

/// Each tool's own variables pass where the parent has them, its fixed
/// and session variables are set whatever the parent has, and none of them
/// is a name no CLI is given.
// Unix only: unix paths and a home found by HOME, as the CLI seat has
// them (Windows sets no HOME and joins with \; not yet supported there).
#[cfg(unix)]
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
            .env(
                Path::new("/s/tmp"),
                Path::new("/s/support"),
                Some(Path::new("/s/store")),
            )
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
    assert_eq!(
        opencode["OPENCODE_DB"], "/s/store/opencode.db",
        "its conversations in the seat's store, never the user's"
    );
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

mod codex;
mod junie;

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
    assert!(
        !fixed.contains_key("OPENCODE_DB"),
        "the store names it ({:?})",
        Opencode.store_env(Path::new("/s/store"))
    );
}

/// opencode goes on with a conversation by the id its lines name: an
/// explicit `--session <id>` (never `--continue`), from a database in the
/// seat's store, an absolute path. An id that could be read as a flag is
/// no id.
// Unix only: unix paths and a home found by HOME, as the CLI seat has
// them (Windows sets no HOME and joins with \; not yet supported there).
#[cfg(unix)]
#[test]
fn opencode_resumes_by_the_id_its_lines_name() {
    assert!(Opencode.resumes());
    let store = Opencode.store_env(Path::new("/s/store"));
    assert_eq!(store.len(), 1);
    assert_eq!(store[0].0, "OPENCODE_DB");
    assert_eq!(store[0].1, OsString::from("/s/store/opencode.db"));
    let mut args = args_of(&Opencode, None, None);
    let fresh = args.len();
    let mut resumed: Vec<OsString> = args.drain(..).map(OsString::from).collect();
    Opencode.resume(&mut resumed, "ses_1");
    assert_eq!(resumed.len(), fresh + 2);
    assert_eq!(
        resumed[fresh..],
        [OsString::from("--session"), OsString::from("ses_1")]
    );
    let mut wire = Wire::default();
    for line in [
        r#"{"type":"step_start","timestamp":1,"sessionID":"ses_1","part":{}}"#,
        r#"{"type":"text","timestamp":2,"sessionID":"ses_2","part":{"text":""}}"#,
    ] {
        Opencode.read_event(line, &mut wire);
    }
    assert_eq!(wire.conversation.as_deref(), Some("ses_1"), "its first");
    let mut wire = Wire::default();
    Opencode.read_event(
        r#"{"type":"step_start","timestamp":1,"sessionID":"--continue"}"#,
        &mut wire,
    );
    assert_eq!(wire.conversation, None);
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
    let init = r#"{"event":"init","conversation_id":"c3b6","init":{"agent":"baylee-seat","cwd":"/w","tools":["ask_permission","run_command","write_to_file"],"permission_mode":"request-review"}}"#;
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

// ---------------------------------------------------------------------
// Resuming beside the login: Codex
// ---------------------------------------------------------------------

/// A directory of the test's own, empty.
fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("baylee-dialect-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const THREAD: &str = "0199a213-81c0-7800-8aa1-bbab2a035a53";

/// Codex goes on with a thread by the UUID its `thread.started` named:
/// every flag kept ahead of `resume <id> -`, never `--last`, and a thread
/// named by anything but a UUID (which `resume` would look up as a name)
/// is no id. Nothing is ephemeral, or there would be nothing to resume.
#[test]
fn codex_resumes_its_thread_by_its_uuid() {
    assert!(Codex.resumes() && Codex.usage_spans_resumes());
    let fresh: Vec<OsString> = args_of(&Codex, Some("gpt-6.1-sol"), Some("low"))
        .into_iter()
        .map(OsString::from)
        .collect();
    assert!(!fresh.iter().any(|arg| arg == "--ephemeral"));
    let mut resumed = fresh.clone();
    Codex.resume(&mut resumed, THREAD);
    assert_eq!(resumed[..fresh.len() - 1], fresh[..fresh.len() - 1]);
    assert_eq!(
        resumed[fresh.len() - 1..],
        ["resume", THREAD, "-"].map(OsString::from)
    );
    assert!(!resumed.iter().any(|arg| arg == "--last" || arg == "--all"));
    let named = |line: &str| {
        let mut wire = Wire::default();
        Codex.read_event(line, &mut wire);
        wire.conversation
    };
    assert_eq!(
        named(&format!(
            r#"{{"type":"thread.started","thread_id":"{THREAD}"}}"#
        ))
        .as_deref(),
        Some(THREAD)
    );
    for bad in ["my-thread", "--last", "", "0199a213"] {
        assert_eq!(
            named(&format!(
                r#"{{"type":"thread.started","thread_id":"{bad}"}}"#
            )),
            None,
            "{bad:?}"
        );
    }
    let mut wire = Wire {
        asked: Some(THREAD.into()),
        ..Wire::default()
    };
    Codex.read_event(
        r#"{"type":"thread.started","thread_id":"0199a213-0000-7000-8000-000000000001"}"#,
        &mut wire,
    );
    assert!(wire.strayed(), "another thread than the one asked for");
}

/// Codex's sessions are where its login is (`CODEX_HOME`, else
/// `~/.codex`); the seat's own are exactly the rollout files named for its
/// thread, at their place, never a link, another thread's or a lookalike.
#[cfg(unix)]
#[test]
fn codex_names_exactly_its_threads_rollout_files() {
    let env = |home: Option<&'static str>| {
        move |name: &str| match name {
            "HOME" => Some(OsString::from("/home/t")),
            "CODEX_HOME" => home.map(OsString::from),
            _ => None,
        }
    };
    assert_eq!(
        Codex.sessions_root(&env(None)),
        Some(PathBuf::from("/home/t/.codex/sessions"))
    );
    assert_eq!(
        Codex.sessions_root(&env(Some("/c"))),
        Some(PathBuf::from("/c/sessions"))
    );
    assert_eq!(Codex.sessions_root(&env(Some("relative"))), None);
    let root = scratch_dir("codex-files");
    let day = root.join("2026/10/06");
    std::fs::create_dir_all(&day).unwrap();
    let ours = day.join(format!("rollout-2026-10-06T12-00-00-{THREAD}.jsonl"));
    let packed = root.join("2026/09/01");
    std::fs::create_dir_all(&packed).unwrap();
    let ours_packed = packed.join(format!("rollout-2026-09-01T08-00-00-{THREAD}.jsonl.zst"));
    let other = day.join("rollout-2026-10-06T12-00-00-0199a213-0000-7000-8000-000000000001.jsonl");
    let lookalike = day.join(format!("rollout-2026-10-06T12-00-00-{THREAD}x.jsonl"));
    let shallow = root.join(format!("rollout-2026-10-06T12-00-00-{THREAD}.jsonl"));
    for file in [&ours, &ours_packed, &other, &lookalike, &shallow] {
        std::fs::write(file, "{}").unwrap();
    }
    let link = day.join(format!("rollout-2026-10-06T13-00-00-{THREAD}.jsonl"));
    std::os::unix::fs::symlink(&other, &link).unwrap();
    let mut found = Codex.session_files(&root, THREAD);
    found.sort();
    assert_eq!(found, [ours_packed, ours]);
    assert!(Codex.session_files(&root, "my-thread").is_empty());
    assert!(Codex.session_files(&root, "..").is_empty());
    std::fs::remove_dir_all(&root).unwrap();
}

/// Codex reads `$CODEX_HOME/AGENTS.md` whatever its flags say: the bridge
/// warns where one is there, and says nothing where none is.
#[test]
fn codex_warns_of_an_agents_file_in_its_home() {
    let home = scratch_dir("codex-agents");
    let env = |name: &str| (name == "CODEX_HOME").then(|| home.clone().into_os_string());
    assert_eq!(Codex.home_warning(&env), None);
    std::fs::write(home.join("AGENTS.md"), "be nice").unwrap();
    let warning = Codex.home_warning(&env).expect("a warning");
    assert!(warning.contains("AGENTS.md"), "{warning}");
    for dialect in the_dialects() {
        if dialect.tool() != CliTool::Codex {
            assert_eq!(dialect.home_warning(&env), None);
        }
    }
    std::fs::remove_dir_all(&home).unwrap();
}
