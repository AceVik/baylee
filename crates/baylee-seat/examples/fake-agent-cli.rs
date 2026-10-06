//! A stand-in for an agent CLI, for the seat bridge's tests
//! (`tests/cli_mind.rs`): it speaks Claude Code's stream-json, runs no
//! model and reaches no network. Built with the tests, shipped nowhere.
//!
//! The bridge clears the environment of every process it starts, so the
//! fake is told what to do in a file under `$HOME`, which a test points at
//! a directory of its own: `fake-cli.json`,
//!
//! ```json
//! {"init": {"tools": ["StructuredOutput", "Bash"]}, "init_delay_ms": 300,
//!  "logged_in": true,
//!  "steps": [{"kind": "answer", "answer": {"ask": "q1", "pick": ["p"]}}]}
//! ```
//!
//! `init` changes the `init` line it writes before its first reply, which
//! is otherwise a locked-down process's (only `StructuredOutput`, no MCP
//! server, no slash command, `apiKeySource` `none`): each field given
//! replaces the default's, a `null` leaves it out, and `false` leaves out
//! the whole line. `init_delay_ms` waits before writing it.
//!
//! One step per message, in order across every process it starts (the
//! next step's index is kept in `fake-cli.cursor`): `answer` (the object,
//! as `structured_output`; `usage` and `delay_ms` optional), `long` (a
//! result line of more than a mebibyte, then the answer as `answer`
//! writes it), `text` (a result with only its `text`), `rate_limit`
//! (`lifts_in` seconds optional), `fail` (an error result), `hang` (no
//! reply, ever) and `exit` (`code`, after a line on stderr). Any step with
//! `exit_after` (a code) exits once it has replied, before it reads the
//! next message: a process that dies between two questions. Past the last
//! step it exits.
//!
//! `cumulative_usage: true` reports each reply's usage as the process's
//! running count since it started, as agy does. Started as agy is (its
//! `--input-format` first, no `-p`), it says its `init` and its answers in
//! agy's shape; only `answer` steps.
//!
//! Started as a tool that answers one message a process is (`codex exec`,
//! `opencode run`, Junie's `--input-format=json` first), it reads its
//! stdin to its end as the one message, logs it (`prompt`) and the files
//! in its session's `support` directory (`support`, beside its `TMPDIR`),
//! answers the next step in that tool's shape and exits: `answer`, `text`,
//! `tool` (the model uses a tool, then answers), `rate_limit`, `fail` and
//! `exit` (and, as Junie, `signed_out`: a failure before any session).
//! As opencode it keeps its conversations as its tool does, in the file
//! `OPENCODE_DB` names (a JSON object of each session's turns, standing in
//! for its database; `:memory:` or none keeps nothing): a new one is
//! `ses_fake<pid>`, `--session <id>` goes on with one, and one that is not
//! there, or a file it cannot read, is `Session not found` on stderr and
//! an exit before any line, as opencode's is. It logs the session and its
//! turn (`session`, `turn`), or the one it did not find (`not_found`).
//! As Codex it keeps each thread as a rollout file under `$CODEX_HOME`
//! (else `~/.codex`), `sessions/2026/10/06/rollout-…-<id>.jsonl`, holding
//! its turns and its running count, which `turn.completed` reports whole;
//! `resume <id>` goes on with one, and one not there is an exit before any
//! line. As Junie it keeps `sessions/<id>/events.jsonl` under
//! `$JUNIE_HOME` (else `~/.junie`); `--session-id=<id>` follows one up,
//! and one not there begins a new session silently, as Junie may. With
//! `stray: true` in the script, both begin a new one whatever they are
//! asked to go on with (logged as `strayed`).
//! Each one's login check answers as its tool's does (`login
//! status` on stderr, `auth list`, `--version`), signed in unless
//! `logged_in` is false.
//!
//! It appends to `fake-cli.log`, one JSON object a line, what it was
//! started with (its arguments, its whole environment, its working
//! directory, how many entries that holds and, on Unix, its and its
//! parent's modes), every line it read, its stdin's end, and an exit it
//! chose (`exit`, `exit_after`) as it takes it. `auth status
//! --json` answers `{"loggedIn": logged_in}` and reads nothing.

use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A tool that answers one message a process.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OneShot {
    Codex,
    Opencode,
    Junie,
}

/// The one-shot tool `args` start, and the one whose login check they are.
fn one_shot_of(args: &[String]) -> (Option<OneShot>, Option<OneShot>) {
    let one_shot = match args.get(1).map(String::as_str) {
        Some("exec") => Some(OneShot::Codex),
        Some("run") => Some(OneShot::Opencode),
        Some("--input-format=json") => Some(OneShot::Junie),
        _ => None,
    };
    let probe = match args.get(1..) {
        Some([a, b]) if a == "login" && b == "status" => Some(OneShot::Codex),
        Some([a, b]) if a == "auth" && b == "list" => Some(OneShot::Opencode),
        Some([a]) if a == "--version" => Some(OneShot::Junie),
        _ => None,
    };
    (one_shot, probe)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let auth = args.get(1..) == Some(&["auth".into(), "status".into(), "--json".into()][..]);
    // Claude Code is started with `-p`, agy with its `--input-format`.
    let agy = args.get(1).map(String::as_str) == Some("--input-format");
    let (one_shot, probe) = one_shot_of(&args);
    if !auth
        && !agy
        && one_shot.is_none()
        && probe.is_none()
        && args.get(1).map(String::as_str) != Some("-p")
    {
        // Run as a test binary (`--all-targets`, nextest's listing): it has
        // no tests, and touches nothing.
        return;
    }
    let home = PathBuf::from(std::env::var_os("HOME").expect("the fake needs HOME"));
    let config: Value = std::fs::read(home.join("fake-cli.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_else(|| json!({}));
    let log = home.join("fake-cli.log");
    let pid = std::process::id();
    let env: BTreeMap<String, String> = std::env::vars().collect();
    let cwd = std::env::current_dir().expect("a working directory");
    let entries = std::fs::read_dir(&cwd).map_or(usize::MAX, Iterator::count);
    dump(
        &log,
        &json!({
            "pid": pid,
            "start": {
                "argv": args,
                "env": env,
                "cwd": cwd,
                "cwd_entries": entries,
                "cwd_mode": mode(&cwd),
                "root_mode": cwd.parent().map(mode),
            },
        }),
    );
    if auth {
        let logged_in = config
            .get("logged_in")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        say(&json!({"loggedIn": logged_in, "authMethod": "fake"}));
        std::process::exit(i32::from(!logged_in));
    }
    let logged_in = config
        .get("logged_in")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if let Some(tool) = probe {
        answer_probe(tool, logged_in);
        return;
    }
    if let Some(tool) = one_shot {
        answer_once(tool, &home, &config, &log, pid);
        return;
    }
    let mut init = init(&config, pid).map(|line| if agy { agy_init(&line) } else { line });
    let cumulative = config
        .get("cumulative_usage")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut running = Usage::default();
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        dump(&log, &json!({"pid": pid, "stdin": line}));
        if let Some(init) = init.take() {
            if let Some(ms) = config.get("init_delay_ms").and_then(Value::as_u64) {
                std::thread::sleep(Duration::from_millis(ms));
            }
            say(&init);
        }
        let Some(step) = next_step(&home, &config) else {
            eprintln!("fake: the script ended");
            std::process::exit(0);
        };
        if let Some(ms) = step.get("delay_ms").and_then(Value::as_u64) {
            std::thread::sleep(Duration::from_millis(ms));
        }
        if step.get("kind").and_then(Value::as_str) == Some("exit") {
            dump(&log, &json!({"pid": pid, "exit": step.get("code")}));
        }
        let mut step = step;
        if cumulative {
            running.add(&usage_of(&step));
            step["usage"] = running.json();
        }
        if agy {
            play_agy(&step);
        } else {
            play(&step);
        }
        if let Some(code) = step.get("exit_after").and_then(Value::as_i64) {
            dump(&log, &json!({"pid": pid, "exit": code}));
            eprintln!("fake: exiting with {code} after its reply");
            std::process::exit(i32::try_from(code).unwrap_or(1));
        }
    }
    dump(&log, &json!({"pid": pid, "eof": true}));
}

/// The `init` line, as the script changes it; `None` for none.
fn init(config: &Value, pid: u32) -> Option<Value> {
    let mut init = json!({
        "type": "system", "subtype": "init", "session_id": format!("fake-{pid}"),
        "tools": ["StructuredOutput"], "mcp_servers": [], "slash_commands": [],
        "apiKeySource": "none",
    });
    match config.get("init") {
        Some(Value::Bool(false)) => return None,
        Some(Value::Object(fields)) => {
            let line = init.as_object_mut().expect("an object");
            for (key, value) in fields {
                if value.is_null() {
                    line.remove(key);
                } else {
                    line.insert(key.clone(), value.clone());
                }
            }
        }
        _ => {}
    }
    Some(init)
}

/// A reply's usage when the step names none.
fn default_usage() -> Value {
    json!({"input_tokens": 100, "output_tokens": 20,
           "cache_creation_input_tokens": 0, "cache_read_input_tokens": 1000})
}

/// The usage a step's reply reports on its own: the step's, else the default.
fn usage_of(step: &Value) -> Usage {
    let usage = step.get("usage").cloned().unwrap_or_else(default_usage);
    let n = |key: &str| usage.get(key).and_then(Value::as_u64).unwrap_or(0);
    Usage {
        input: n("input_tokens"),
        output: n("output_tokens"),
        cache_write: n("cache_creation_input_tokens"),
        cache_read: n("cache_read_input_tokens"),
    }
}

/// A process's running count of what it used, for `cumulative_usage`.
#[derive(Clone, Copy, Default)]
struct Usage {
    input: u64,
    output: u64,
    cache_write: u64,
    cache_read: u64,
}

impl Usage {
    fn add(&mut self, other: &Self) {
        self.input += other.input;
        self.output += other.output;
        self.cache_write += other.cache_write;
        self.cache_read += other.cache_read;
    }

    fn json(&self) -> Value {
        json!({"input_tokens": self.input, "output_tokens": self.output,
               "cache_creation_input_tokens": self.cache_write,
               "cache_read_input_tokens": self.cache_read})
    }
}

/// A login check, as each one-shot tool answers it.
fn answer_probe(tool: OneShot, logged_in: bool) {
    match (tool, logged_in) {
        (OneShot::Codex, true) => eprintln!("Logged in using ChatGPT"),
        (OneShot::Codex, false) => {
            eprintln!("Not logged in");
            std::process::exit(1);
        }
        (OneShot::Opencode, true) => {
            println!(
                "Credentials ~/.local/share/opencode/auth.json\n  Anthropic oauth\n1 credentials"
            );
        }
        (OneShot::Opencode, false) => println!("0 credentials"),
        (OneShot::Junie, _) => println!("Junie version: 26.9.22 (fake)"),
    }
}

/// Reads stdin to its end as the one message, answers it as `tool` does
/// with the next step, and exits.
fn answer_once(tool: OneShot, home: &Path, config: &Value, log: &Path, pid: u32) {
    let support = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .and_then(|tmp| tmp.parent().map(|root| root.join("support")));
    let mut files: Vec<String> = support
        .as_deref()
        .and_then(|dir| std::fs::read_dir(dir).ok())
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    files.sort();
    dump(log, &json!({"pid": pid, "support": files}));
    let mut prompt = String::new();
    let _ = std::io::Read::read_to_string(&mut std::io::stdin().lock(), &mut prompt);
    dump(log, &json!({"pid": pid, "prompt": prompt}));
    dump(log, &json!({"pid": pid, "eof": true}));
    let kept = match tool {
        OneShot::Opencode => Kept {
            id: opencode_session(log, pid),
            turn: 0,
            before: Usage::default(),
            file: None,
        },
        OneShot::Codex => codex_thread(config, log, pid),
        OneShot::Junie => junie_session(config, log, pid),
    };
    let Some(step) = next_step(home, config) else {
        eprintln!("fake: the script ended");
        std::process::exit(0);
    };
    if let Some(ms) = step.get("delay_ms").and_then(Value::as_u64) {
        std::thread::sleep(Duration::from_millis(ms));
    }
    let kind = step.get("kind").and_then(Value::as_str).unwrap_or("answer");
    if kind == "exit" {
        let code = step.get("code").and_then(Value::as_i64).unwrap_or(1);
        dump(log, &json!({"pid": pid, "exit": code}));
        eprintln!("fake: exiting with {code}");
        std::process::exit(i32::try_from(code).unwrap_or(1));
    }
    let usage = usage_of(&step);
    let mut total = kept.before;
    total.add(&usage);
    if let Some(file) = &kept.file {
        // Kept before a word is said, as the tool keeps it.
        let record = json!({"turn": kept.turn, "input": total.input, "output": total.output,
            "cache_write": total.cache_write, "cache_read": total.cache_read});
        std::fs::write(file, record.to_string()).expect("the fake's session is written");
    }
    let session = kept.id;
    let answer = match kind {
        "text" => step
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        _ => step
            .get("answer")
            .cloned()
            .unwrap_or(Value::Null)
            .to_string(),
    };
    match tool {
        // Codex reports the thread's running count, seeded from its
        // rollout.
        OneShot::Codex => play_codex(kind, &answer, &total, &session),
        OneShot::Opencode => play_opencode(kind, &answer, &usage, &session),
        OneShot::Junie => play_junie(kind, &answer, &usage, &session),
    }
    if matches!(kind, "rate_limit" | "fail") {
        std::process::exit(1);
    }
}

/// One message answered as `codex exec --json` does.
fn play_codex(kind: &str, answer: &str, usage: &Usage, thread: &str) {
    say(&json!({"type": "thread.started", "thread_id": thread}));
    say(&json!({"type": "turn.started"}));
    match kind {
        "rate_limit" => {
            say(&json!({"type": "error", "message": "Reconnecting... 1/5"}));
            say(&json!({"type": "turn.failed", "error": {"message":
                "You've hit your usage limit. Try again at 3:04 PM."}}));
            return;
        }
        "fail" => {
            say(&json!({"type": "turn.failed", "error": {"message": "the fake failed"}}));
            return;
        }
        "tool" => say(&json!({"type": "item.started", "item": {"id": "item_1",
            "type": "command_execution", "command": "bash -lc ls", "status": "in_progress"}})),
        _ => {}
    }
    say(
        &json!({"type": "item.completed", "item": {"id": "item_0", "type": "reasoning",
        "text": "thinking"}}),
    );
    say(
        &json!({"type": "item.completed", "item": {"id": "item_3", "type": "agent_message",
        "text": answer}}),
    );
    say(&json!({"type": "turn.completed", "usage": {
        "input_tokens": usage.input + usage.cache_read, "cached_input_tokens": usage.cache_read,
        "output_tokens": usage.output, "reasoning_output_tokens": 0}}));
}

/// The conversation a process answers in, as its tool keeps it.
struct Kept {
    id: String,
    /// Its turn, this one counted.
    turn: u64,
    /// Its running count before this turn.
    before: Usage,
    /// Where it is kept after this turn, if the fake keeps it there.
    file: Option<PathBuf>,
}

/// A tool's home: `name`'s value, else `fallback` under `HOME`.
fn tool_home(name: &str, fallback: &str) -> PathBuf {
    std::env::var_os(name).map_or_else(
        || PathBuf::from(std::env::var_os("HOME").expect("HOME")).join(fallback),
        PathBuf::from,
    )
}

/// What a kept session file says: its turns and its running count.
fn read_kept(file: &Path) -> Option<(u64, Usage)> {
    let value: Value = serde_json::from_slice(&std::fs::read(file).ok()?).ok()?;
    let n = |key: &str| value.get(key).and_then(Value::as_u64);
    Some((
        n("turn")?,
        Usage {
            input: n("input")?,
            output: n("output")?,
            cache_write: n("cache_write")?,
            cache_read: n("cache_read")?,
        },
    ))
}

/// Whether the script makes every resume begin a new conversation.
fn strays(config: &Value) -> bool {
    config.get("stray").and_then(Value::as_bool) == Some(true)
}

/// The Codex thread this process answers in: the one `resume <id>` names,
/// from its rollout file, or a new one. One not there ends the process
/// before any line.
fn codex_thread(config: &Value, log: &Path, pid: u32) -> Kept {
    let args: Vec<String> = std::env::args().collect();
    let day = tool_home("CODEX_HOME", ".codex").join("sessions/2026/10/06");
    std::fs::create_dir_all(&day).expect("the fake's sessions directory");
    let rollout = |id: &str| day.join(format!("rollout-2026-10-06T12-00-00-{id}.jsonl"));
    let asked = args
        .iter()
        .position(|arg| arg == "resume")
        .and_then(|at| args.get(at + 1))
        .cloned();
    let fresh = format!("0199a213-0000-7000-8000-{pid:012}");
    let (id, turn, before) = match asked {
        Some(_) if strays(config) => {
            dump(log, &json!({"pid": pid, "strayed": fresh}));
            (fresh, 1, Usage::default())
        }
        Some(id) => {
            let Some((turn, before)) = read_kept(&rollout(&id)) else {
                dump(log, &json!({"pid": pid, "not_found": id}));
                eprintln!("Error: thread/resume failed: no rollout found for thread id {id}");
                std::process::exit(1);
            };
            (id, turn + 1, before)
        }
        None => (fresh, 1, Usage::default()),
    };
    dump(log, &json!({"pid": pid, "session": id, "turn": turn}));
    Kept {
        file: Some(rollout(&id)),
        id,
        turn,
        before,
    }
}

/// The Junie session this process answers in: the one `--session-id`
/// names, or, when that one is not there, a new one, silently.
fn junie_session(config: &Value, log: &Path, pid: u32) -> Kept {
    let sessions = tool_home("JUNIE_HOME", ".junie").join("sessions");
    let asked =
        std::env::args().find_map(|arg| arg.strip_prefix("--session-id=").map(str::to_string));
    let events = |id: &str| sessions.join(id).join("events.jsonl");
    let going = asked
        .filter(|_| !strays(config))
        .and_then(|id| read_kept(&events(&id)).map(|(turn, _)| (id, turn + 1)));
    let (id, turn) = going.unwrap_or_else(|| {
        let fresh = format!("session-261006-120000-{pid}");
        if strays(config) {
            dump(log, &json!({"pid": pid, "strayed": fresh}));
        }
        (fresh, 1)
    });
    std::fs::create_dir_all(sessions.join(&id)).expect("the fake's session directory");
    dump(log, &json!({"pid": pid, "session": id, "turn": turn}));
    Kept {
        file: Some(events(&id)),
        id,
        turn,
        before: Usage::default(),
    }
}

/// The opencode session this process answers in: the one `--session`
/// names, from the store `OPENCODE_DB` names, or a new one kept there. One
/// it cannot find ends the process as opencode does, before any line.
fn opencode_session(log: &Path, pid: u32) -> String {
    let args: Vec<String> = std::env::args().collect();
    let named = args
        .iter()
        .position(|arg| arg == "--session")
        .and_then(|at| args.get(at + 1))
        .cloned();
    let store = std::env::var_os("OPENCODE_DB")
        .filter(|db| db != ":memory:")
        .map(PathBuf::from);
    let mut sessions: Value = match &store {
        Some(db) if db.exists() => std::fs::read(db)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or(Value::Null),
        _ => json!({}),
    };
    let id = if let Some(id) = named {
        if sessions.get(&id).and_then(Value::as_u64).is_none() {
            dump(log, &json!({"pid": pid, "not_found": id}));
            eprintln!("Error: Session not found");
            std::process::exit(1);
        }
        id
    } else {
        if !sessions.is_object() {
            sessions = json!({});
        }
        format!("ses_fake{pid}")
    };
    let turn = sessions.get(&id).and_then(Value::as_u64).unwrap_or(0) + 1;
    sessions[&id] = json!(turn);
    if let Some(db) = &store {
        std::fs::write(db, sessions.to_string()).expect("the fake's store is written");
    }
    dump(log, &json!({"pid": pid, "session": id, "turn": turn}));
    id
}

/// One message answered as `opencode run --format json` does, in
/// `session`.
fn play_opencode(kind: &str, answer: &str, usage: &Usage, session: &str) {
    let line = |kind: &str, body: Value| {
        let mut line = json!({"type": kind, "timestamp": 1, "sessionID": session});
        line.as_object_mut()
            .expect("an object")
            .extend(body.as_object().expect("an object").clone());
        say(&line);
    };
    match kind {
        "rate_limit" => {
            line(
                "error",
                json!({"error": {"name": "APIError", "data": {
                "message": "Too Many Requests", "statusCode": 429, "isRetryable": true}}}),
            );
            return;
        }
        "fail" => {
            line(
                "error",
                json!({"error": {"name": "UnknownError", "data": {
                "message": "the fake failed"}}}),
            );
            return;
        }
        _ => {}
    }
    line(
        "step_start",
        json!({"part": {"id": "prt_1", "type": "step-start"}}),
    );
    if kind == "tool" {
        line(
            "tool_use",
            json!({"part": {"type": "tool", "callID": "c1", "tool": "bash",
            "state": {"status": "completed", "output": ""}}}),
        );
    }
    line(
        "text",
        json!({"part": {"type": "text", "text": answer,
        "time": {"start": 1, "end": 2}}}),
    );
    line(
        "step_finish",
        json!({"part": {"type": "step-finish", "reason": "stop", "cost": 0,
        "tokens": {"input": usage.input, "output": usage.output, "reasoning": 0,
                   "cache": {"read": usage.cache_read, "write": usage.cache_write}}}}),
    );
}

/// One task answered as Junie's `json-stream` does, its banner first.
fn play_junie(kind: &str, answer: &str, usage: &Usage, session: &str) {
    println!("Junie fake banner");
    if kind == "signed_out" {
        // Its first word, before any session: it cannot sign in.
        say(&json!({"type": "result", "timestamp": 1, "result": "",
            "errors": ["Cannot find authorization"]}));
        std::process::exit(1);
    }
    say(&json!({"type": "session", "timestamp": 1, "sessionId": session}));
    let errors = match kind {
        "rate_limit" => {
            vec!["Junie: Insufficient Account Balance. All tokens on your balance are spent."]
        }
        "fail" => vec!["the fake failed"],
        _ => Vec::new(),
    };
    if kind == "tool" {
        say(
            &json!({"type": "step", "timestamp": 2, "name": "Opened file",
            "details": "ping.txt"}),
        );
    }
    if errors.is_empty() {
        say(&json!({"type": "step", "timestamp": 3, "name": "TASK RESULT", "details": answer}));
    }
    say(
        &json!({"type": "result", "timestamp": 4, "result": if errors.is_empty() { answer } else { "" },
        "errors": errors, "changes": [],
        "errorCode": [{"model": "fake", "calls": 1, "cost": 0.0,
            "inputTokens": usage.input, "cacheInputTokens": usage.cache_read,
            "cacheCreateTokens": usage.cache_write, "outputTokens": usage.output}]}),
    );
}

/// Claude Code's `init` line as agy says it.
fn agy_init(line: &Value) -> Value {
    json!({"event": "init", "init": line})
}

/// Answers one message as agy would: only `answer` steps, with agy's
/// `cache_read_tokens` for Claude Code's `cache_read_input_tokens`.
fn play_agy(step: &Value) {
    let kind = step.get("kind").and_then(Value::as_str).unwrap_or("answer");
    assert_eq!(kind, "answer", "the fake speaks only answers as agy");
    let answer = step.get("answer").cloned().unwrap_or(Value::Null);
    let usage = step.get("usage").cloned().unwrap_or_else(default_usage);
    let n = |key: &str| usage.get(key).and_then(Value::as_u64).unwrap_or(0);
    say(&json!({"event": "result", "result": {
        "status": "SUCCESS", "response": answer.to_string(), "structured_output": answer,
        "usage": {"input_tokens": n("input_tokens"), "output_tokens": n("output_tokens"),
                  "cache_read_tokens": n("cache_read_input_tokens")}}}));
}

/// Answers one message as `step` says.
fn play(step: &Value) {
    let usage = step.get("usage").cloned().unwrap_or_else(default_usage);
    match step.get("kind").and_then(Value::as_str).unwrap_or("answer") {
        "answer" => {
            let answer = step.get("answer").cloned().unwrap_or(Value::Null);
            say(
                &json!({"type": "assistant", "message": {"role": "assistant",
                        "content": [{"type": "text", "text": "thinking"}]}}),
            );
            say(
                &json!({"type": "result", "subtype": "success", "is_error": false,
                        "result": answer.to_string(), "structured_output": answer,
                        "usage": usage}),
            );
        }
        "long" => {
            let padding = "x".repeat((1 << 20) + 16);
            say(
                &json!({"type": "result", "subtype": "success", "is_error": false,
                        "result": padding, "structured_output": {"ask": "q0", "pick": ["none"]},
                        "usage": usage}),
            );
            play(&json!({"kind": "answer", "answer": step.get("answer")}));
        }
        "text" => {
            let text = step.get("text").and_then(Value::as_str).unwrap_or_default();
            say(
                &json!({"type": "result", "subtype": "success", "is_error": false,
                        "result": text, "usage": usage}),
            );
        }
        "rate_limit" => {
            let mut text = "Claude AI usage limit reached".to_string();
            if let Some(secs) = step.get("lifts_in").and_then(Value::as_u64) {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                text = format!("{text}|{}", now + secs);
            }
            say(
                &json!({"type": "assistant", "error": "rate_limit", "message": {"role": "assistant",
                        "content": [{"type": "text", "text": text}]}}),
            );
            say(&json!({"type": "result", "subtype": "success", "is_error": true, "result": text}));
        }
        "fail" => say(
            &json!({"type": "result", "subtype": "error_during_execution",
                              "is_error": true, "result": "the fake failed"}),
        ),
        "hang" => std::thread::sleep(Duration::from_secs(600)),
        "exit" => {
            let code = step.get("code").and_then(Value::as_i64).unwrap_or(1);
            eprintln!("fake: exiting with {code}");
            std::process::exit(i32::try_from(code).unwrap_or(1));
        }
        other => panic!("the fake has no step {other}"),
    }
}

/// The script's next step, past the steps every earlier process took.
fn next_step(home: &Path, config: &Value) -> Option<Value> {
    let cursor = home.join("fake-cli.cursor");
    let at: usize = std::fs::read_to_string(&cursor)
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0);
    std::fs::write(&cursor, (at + 1).to_string()).expect("the cursor is written");
    config.get("steps")?.get(at).cloned()
}

/// One line on stdout, at once.
fn say(value: &Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{value}");
    let _ = out.flush();
}

/// One line on the log, appended whole.
fn dump(log: &Path, value: &Value) {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .expect("the log opens");
    let _ = file.write_all(format!("{value}\n").as_bytes());
}

/// A directory's permission bits, as octal; `None` off Unix.
#[cfg(unix)]
fn mode(path: &Path) -> Option<String> {
    use std::os::unix::fs::PermissionsExt;
    let meta = std::fs::metadata(path).ok()?;
    Some(format!("{:o}", meta.permissions().mode() & 0o777))
}

#[cfg(not(unix))]
fn mode(_: &Path) -> Option<String> {
    None
}
