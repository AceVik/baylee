//! A stand-in for an agent CLI, for the seat bridge's tests
//! (`tests/cli_mind.rs`): it speaks Claude Code's stream-json, runs no
//! model and reaches no network. Built with the tests, shipped nowhere.
//!
//! The bridge clears the environment of every process it starts, so the
//! fake is told what to do in a file under `$HOME`, which a test points at
//! a directory of its own: `fake-cli.json`,
//!
//! ```json
//! {"init_tools": ["StructuredOutput"], "logged_in": true,
//!  "steps": [{"kind": "answer", "answer": {"ask": "q1", "pick": ["p"]}}]}
//! ```
//!
//! with one step per message, in order across every process it starts
//! (the next step's index is kept in `fake-cli.cursor`): `answer` (the
//! object, as `structured_output`; `usage` and `delay_ms` optional),
//! `text` (a result with only its `text`), `rate_limit` (`lifts_in` seconds
//! optional), `fail` (an error result), `hang` (no reply, ever) and `exit`
//! (`code`, after a line on stderr). Past the last step it exits.
//!
//! It appends to `fake-cli.log`, one JSON object a line, what it was
//! started with (its arguments, its whole environment, its working
//! directory, how many entries that holds and, on Unix, its and its
//! parent's modes), every line it read, and its stdin's end. `auth status
//! --json` answers `{"loggedIn": …}` and reads nothing.

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
    let tools = config
        .get("init_tools")
        .cloned()
        .unwrap_or_else(|| json!(["StructuredOutput"]));
    let mut started = false;
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        dump(&log, &json!({"pid": pid, "stdin": line}));
        if !started {
            started = true;
            say(&json!({
                "type": "system", "subtype": "init", "session_id": format!("fake-{pid}"),
                "tools": tools, "mcp_servers": config.get("init_mcp").cloned().unwrap_or_else(|| json!([])),
            }));
        }
        let Some(step) = next_step(&home, &config) else {
            eprintln!("fake: the script ended");
            std::process::exit(0);
        };
        if let Some(ms) = step.get("delay_ms").and_then(Value::as_u64) {
            std::thread::sleep(Duration::from_millis(ms));
        }
        play(&step);
    }
    dump(&log, &json!({"pid": pid, "eof": true}));
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
