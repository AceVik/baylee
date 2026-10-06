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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let auth = args.get(1..) == Some(&["auth".into(), "status".into(), "--json".into()][..]);
    if !auth && args.get(1).map(String::as_str) != Some("-p") {
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
    let mut init = init(&config, pid);
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
        play(&step);
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

/// Answers one message as `step` says.
fn play(step: &Value) {
    let usage = step.get("usage").cloned().unwrap_or_else(|| {
        json!({"input_tokens": 100, "output_tokens": 20,
               "cache_creation_input_tokens": 0, "cache_read_input_tokens": 1000})
    });
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
