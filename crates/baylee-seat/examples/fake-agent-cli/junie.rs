//! Junie in its ACP mode (`--acp=true`): JSON-RPC lines both ways, one
//! session a process, as `junie` 26.10.27 spoke it on 31.10.2026.
//!
//! `initialize` and `session/new` are answered as Junie answers them (the
//! session kept as `sessions/<id>/events.jsonl` under `$JUNIE_HOME`, else
//! `~/.junie`, its options `mode`, `model` and `effort` as the flags chose
//! and `brave_mode` `brave-auto`); `session/set_config_option` sets an
//! option, except brave mode where the script says `"brave": "stuck"`.
//! `"signed_out": true` fails `session/new` as a missing sign-in does.
//! Each `session/prompt` takes the next step: `answer` (in two chunks,
//! with prose after it whose braces are no answer), `text`, `permission`
//! (a terminal command asked for: it waits for the seat's answer, logs it
//! as `permission_answer`, and runs the command unless that answer refused
//! it, then answers), `tool` (a file read without asking, then the
//! answer), `rate_limit` (a spent balance), `fail`, `hang` and `exit`.

use super::{BufRead, Path, Value, dump, json, next_step, say, tool_home, usage_of};

/// The session's options, as the fake keeps them.
struct Options {
    model: String,
    effort: String,
    brave: String,
    /// Whether brave mode stays as it is, whatever the seat asks.
    stuck: bool,
}

impl Options {
    fn json(&self) -> Value {
        let select = |id: &str, current: &str, values: &[&str]| {
            let options: Vec<Value> = values
                .iter()
                .map(|value| json!({"value": value, "name": value}))
                .collect();
            json!({"type": "select", "id": id, "name": id, "currentValue": current,
                   "options": options})
        };
        json!([
            select("mode", "default", &["default", "plan"]),
            select(
                "model",
                &self.model,
                &["mix.normal", "gemini-3.7-flash", "fake-model"]
            ),
            select(
                "effort",
                &self.effort,
                &["none", "low", "medium", "high", "xhigh", "max"]
            ),
            select("brave_mode", &self.brave, &["brave-auto", "on", "off"]),
        ])
    }
}

/// Answers the seat until its stdin ends.
pub(super) fn serve(home: &Path, config: &Value, log: &Path, pid: u32) {
    let args: Vec<String> = std::env::args().collect();
    let flag = |name: &str| {
        args.iter()
            .find_map(|arg| arg.strip_prefix(name).map(str::to_string))
    };
    let mut options = Options {
        model: flag("--model=").unwrap_or_else(|| "mix.normal".into()),
        effort: flag("--effort=").unwrap_or_else(|| "high".into()),
        brave: "brave-auto".into(),
        stuck: config.get("brave").and_then(Value::as_str) == Some("stuck"),
    };
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    let mut session = String::new();
    while let Some(Ok(line)) = lines.next() {
        dump(log, &json!({"pid": pid, "stdin": line}));
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let id = message.get("id").cloned().unwrap_or(Value::Null);
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        match message.get("method").and_then(Value::as_str) {
            Some("initialize") => say(&json!({"jsonrpc": "2.0", "id": id, "result": {
                "protocolVersion": 1,
                "agentCapabilities": {"loadSession": true},
                "authMethods": [],
            }})),
            Some("session/new") => {
                if config.get("signed_out").and_then(Value::as_bool) == Some(true) {
                    say(&json!({"jsonrpc": "2.0", "id": id, "error": {
                        "code": -32000, "message": "Authentication required"}}));
                    continue;
                }
                session = format!("session-261031-120000-{pid}");
                let dir = tool_home("JUNIE_HOME", ".junie")
                    .join("sessions")
                    .join(&session);
                std::fs::create_dir_all(&dir).expect("the fake's session directory");
                std::fs::write(dir.join("events.jsonl"), "{}").expect("the fake's session");
                dump(
                    log,
                    &json!({"pid": pid, "session": session, "cwd_asked": params["cwd"]}),
                );
                say(&json!({"jsonrpc": "2.0", "id": id, "result": {
                    "sessionId": session, "configOptions": options.json()}}));
                say(&update(
                    &session,
                    &json!({"sessionUpdate": "available_commands_update",
                    "availableCommands": [{"name": "plan"}]}),
                ));
            }
            Some("session/set_config_option") => {
                let value = params["value"].as_str().unwrap_or_default().to_string();
                match params["configId"].as_str() {
                    Some("brave_mode") if !options.stuck => options.brave = value,
                    Some("model") => options.model = value,
                    Some("effort") => options.effort = value,
                    _ => {}
                }
                say(&json!({"jsonrpc": "2.0", "id": id, "result": {
                    "configOptions": options.json()}}));
            }
            Some("session/prompt") => {
                let text = params["prompt"][0]["text"].as_str().unwrap_or_default();
                dump(
                    log,
                    &json!({"pid": pid, "prompt": text, "brave": options.brave}),
                );
                let Some(step) = next_step(home, config) else {
                    eprintln!("fake: the script ended");
                    std::process::exit(0);
                };
                play(&step, &id, &session, &mut lines, log, pid);
            }
            Some(other) if !id.is_null() => say(&json!({"jsonrpc": "2.0", "id": id, "error": {
                "code": -32601, "message": format!("no {other}")}})),
            _ => {}
        }
    }
    dump(log, &json!({"pid": pid, "eof": true}));
}

/// A `session/update` of `update` in `session`.
fn update(session: &str, update: &Value) -> Value {
    json!({"jsonrpc": "2.0", "method": "session/update",
           "params": {"sessionId": session, "update": update}})
}

/// A tool call's news.
fn call(session: &str, which: &str, kind: &str, status: &str, title: &str) -> Value {
    update(
        session,
        &json!({"sessionUpdate": which, "toolCallId": "call-1", "title": title,
                "kind": kind, "status": status, "rawInput": {"command": title}}),
    )
}

/// Answers the prompt `id` as `step` says.
fn play(
    step: &Value,
    id: &Value,
    session: &str,
    lines: &mut impl Iterator<Item = std::io::Result<String>>,
    log: &Path,
    pid: u32,
) {
    if let Some(ms) = step.get("delay_ms").and_then(Value::as_u64) {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
    let kind = step.get("kind").and_then(Value::as_str).unwrap_or("answer");
    let error = |message: &str, data: Value| {
        say(&json!({"jsonrpc": "2.0", "id": id, "error": {
            "code": -32603, "message": message, "data": data}}));
    };
    match kind {
        "rate_limit" => {
            return error(
                "Insufficient account balance. All tokens in your account have been spent.",
                json!({"reason": "insufficient_account_balance"}),
            );
        }
        "fail" => return error("the fake failed", Value::Null),
        "hang" => std::thread::sleep(std::time::Duration::from_secs(600)),
        "exit" => {
            let code = step.get("code").and_then(Value::as_i64).unwrap_or(1);
            dump(log, &json!({"pid": pid, "exit": code}));
            eprintln!("fake: exiting with {code}");
            std::process::exit(i32::try_from(code).unwrap_or(1));
        }
        "permission" => ask_permission(session, lines, log, pid),
        "tool" => {
            let file = "/home/x/.junie/settings.json";
            say(&call(session, "tool_call", "read", "in_progress", file));
            say(&call(
                session,
                "tool_call_update",
                "read",
                "completed",
                file,
            ));
        }
        _ => {}
    }
    answer(step, kind, id, session);
}

/// Asks the seat before a terminal command, as Junie with brave mode off
/// does, and runs it unless the seat refused.
fn ask_permission(
    session: &str,
    lines: &mut impl Iterator<Item = std::io::Result<String>>,
    log: &Path,
    pid: u32,
) {
    let command = "ls -la ~/.junie";
    say(&call(session, "tool_call", "execute", "pending", command));
    say(&call(
        session,
        "tool_call_update",
        "execute",
        "in_progress",
        command,
    ));
    say(
        &json!({"jsonrpc": "2.0", "id": 900, "method": "session/request_permission",
        "params": {"sessionId": session,
            "toolCall": {"toolCallId": "call-1", "title": command, "kind": "execute",
                         "status": "pending", "rawInput": {"command": command}},
            "options": [
                {"optionId": "Yes", "name": "Yes", "kind": "allow_once"},
                {"optionId": "No", "name": "No", "kind": "reject_once"},
                {"optionId": "Never", "name": "Never", "kind": "reject_always"},
            ]}}),
    );
    let answer = lines
        .filter_map(Result::ok)
        .inspect(|line| dump(log, &json!({"pid": pid, "stdin": line})))
        .filter_map(|line| serde_json::from_str::<Value>(&line).ok())
        .find(|line| line["id"] == 900 && line.get("method").is_none())
        .unwrap_or(Value::Null);
    dump(log, &json!({"pid": pid, "permission_answer": answer}));
    let outcome = &answer["result"]["outcome"];
    let refused = matches!(outcome["optionId"].as_str(), Some("No" | "Never"))
        || outcome["outcome"] == "cancelled";
    let status = if refused { "failed" } else { "completed" };
    say(&call(
        session,
        "tool_call_update",
        "execute",
        status,
        command,
    ));
}

/// The reply to the prompt `id`: its text in two chunks (an answer with
/// prose after it), what the session cost, and the turn's own usage.
fn answer(step: &Value, kind: &str, id: &Value, session: &str) {
    let usage = usage_of(step);
    let said = match kind {
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
    let half = (said.len() / 2..=said.len())
        .find(|at| said.is_char_boundary(*at))
        .unwrap_or(said.len());
    let (head, tail) = said.split_at(half);
    let chunk = |text: &str| {
        say(&update(
            session,
            &json!({"sessionUpdate": "agent_message_chunk",
                    "content": {"type": "text", "text": text}}),
        ));
    };
    chunk(head);
    chunk(tail);
    if kind != "text" {
        chunk("\n\n### Changes\n- answered {it}, as asked");
    }
    say(&update(
        session,
        &json!({"sessionUpdate": "usage_update", "used": 2000, "size": 1_048_576,
                "cost": {"amount": 0.01, "currency": "USD"}}),
    ));
    let input = usage.input + usage.cache_read + usage.cache_write;
    say(
        &json!({"jsonrpc": "2.0", "id": id, "result": {"stopReason": "end_turn", "usage": {
        "inputTokens": input, "outputTokens": usage.output, "totalTokens": input + usage.output,
        "cachedReadTokens": usage.cache_read, "cachedWriteTokens": usage.cache_write}}}),
    );
}
