//! Junie's ACP: its arguments, the session it opens, and every line of a
//! turn, from the lines `junie --acp=true` (26.10.27) wrote on 31.10.2026
//! and the protocol's own shapes.

use super::*;

/// Junie's side of a session as the seat drives it: each line read as a
/// process's reader reads it, and what the seat wrote back.
struct Drive {
    wire: Wire,
}

impl Drive {
    /// A session opened in `/w` with `effort` and `model`: the greeting.
    fn new(model: Option<&str>, effort: Option<&str>) -> (Self, Vec<Value>) {
        let mut wire = Wire::default();
        let settings = settings_for("junie", effort);
        let lines = Junie.opening(&settings, model, Path::new("/w"), &mut wire);
        (Self { wire }, parsed(&lines))
    }

    /// Reads `line`; the events, and the lines written back.
    fn read(&mut self, line: &Value) -> (Vec<Event>, Vec<Value>) {
        let line = line.to_string();
        let mut events = vec![Junie.read_event(&line, &mut self.wire)];
        if std::mem::take(&mut self.wire.again) {
            events.push(Junie.read_event(&line, &mut self.wire));
        }
        (events, parsed(&std::mem::take(&mut self.wire.out)))
    }

    /// Sends `text`: the lines written.
    fn send(&mut self, text: &str) -> Vec<Value> {
        parsed(&Junie.message(text, &mut self.wire))
    }

    /// Opened through the whole handshake (brave mode asked off and
    /// confirmed), with `first` sent: the session, and its prompt.
    fn opened(first: &str) -> (Self, Value) {
        let (mut drive, _) = Self::new(None, None);
        assert!(
            drive.send(first).is_empty(),
            "held until the session is open"
        );
        drive.read(&initialized());
        drive.read(&created(&[("brave_mode", "brave-auto")]));
        let (_, out) = drive.read(&set_result(3, &[("brave_mode", "off")]));
        assert_eq!(out.len(), 1, "{out:?}");
        drive.wire.notes.clear();
        (drive, out[0].clone())
    }
}

fn parsed(lines: &[String]) -> Vec<Value> {
    lines
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

const SESSION: &str = "session-261031-183427-1c5z";

/// `initialize`'s result, as Junie gave it.
fn initialized() -> Value {
    json!({"jsonrpc": "2.0", "id": 1, "result": {"protocolVersion": 1,
        "agentCapabilities": {"loadSession": true, "promptCapabilities": {"image": true}},
        "authMethods": []}})
}

/// A config option as Junie names it: a select with its current value.
fn option(id: &str, current: &str, values: &[&str]) -> Value {
    let options: Vec<Value> = values
        .iter()
        .map(|value| json!({"value": value, "name": value}))
        .collect();
    json!({"type": "select", "id": id, "name": id, "currentValue": current, "options": options})
}

/// Junie's four options, `brave_mode` and any other as `changed` says.
fn options(changed: &[(&str, &str)]) -> Value {
    let value = |id: &str, default: &'static str| {
        changed
            .iter()
            .find(|(named, _)| *named == id)
            .map_or(default, |(_, value)| value)
            .to_string()
    };
    json!([
        option("mode", &value("mode", "default"), &["default", "plan"]),
        {"type": "select", "id": "model", "currentValue": value("model", "mix.normal"),
         "options": [{"group": "mixes", "options": [{"value": "mix.normal"}]},
                     {"group": "models", "options": [{"value": "gemini-3.7-flash"},
                                                     {"value": "claude-opus-5-5"}]}]},
        option("effort", &value("effort", "high"),
               &["none", "low", "medium", "high", "xhigh", "max"]),
        option("brave_mode", &value("brave_mode", "brave-auto"), &["brave-auto", "on", "off"]),
    ])
}

/// `session/new`'s result (asked as id 2).
fn created(changed: &[(&str, &str)]) -> Value {
    json!({"jsonrpc": "2.0", "id": 2, "result": {"sessionId": SESSION,
        "configOptions": options(changed)}})
}

/// `session/set_config_option`'s result to request `id`: every option, as
/// `changed` says.
fn set_result(id: u64, changed: &[(&str, &str)]) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": {"configOptions": options(changed)}})
}

/// A `session/update` of `update`, in `session`.
fn update_in(session: &str, update: &Value) -> Value {
    json!({"jsonrpc": "2.0", "method": "session/update",
           "params": {"sessionId": session, "update": update}})
}

fn update(update: &Value) -> Value {
    update_in(SESSION, update)
}

fn chunk(text: &str) -> Value {
    update(&json!({"sessionUpdate": "agent_message_chunk",
                   "content": {"type": "text", "text": text}, "messageId": "m1"}))
}

fn tool(kind: &str, status: &str, id: &str, title: &str) -> Value {
    let which = if status == "pending" {
        "tool_call"
    } else {
        "tool_call_update"
    };
    update(
        &json!({"sessionUpdate": which, "toolCallId": id, "title": title, "kind": kind,
                   "status": status, "rawInput": {"command": title}}),
    )
}

/// Junie's permission request, as it asked before `ls`.
fn permission(id: u64, call: &str, options: &Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "session/request_permission", "params": {
        "sessionId": SESSION,
        "toolCall": {"toolCallId": call, "title": "ls -la ~/.junie", "kind": "execute",
                     "status": "pending", "rawInput": {"command": "ls -la ~/.junie"}},
        "options": options}})
}

fn permission_options() -> Value {
    json!([
        {"optionId": "Yes", "name": "Yes", "kind": "allow_once"},
        {"optionId": "Always", "name": "Always", "kind": "allow_always"},
        {"optionId": "Never", "name": "Never", "kind": "reject_always"},
        {"optionId": "No", "name": "No", "kind": "reject_once"},
    ])
}

/// The prompt's end, as Junie gave it.
fn done(id: u64, stop: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": {"stopReason": stop, "usage": {
        "inputTokens": 33640, "outputTokens": 1345, "totalTokens": 34985,
        "cachedReadTokens": 0, "cachedWriteTokens": 0}}})
}

fn failed(id: u64, message: &str, data: &Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32603, "message": message,
                                                 "data": data}})
}

/// `junie --acp=true` with every default location off, the session's own
/// empty guidelines, extensions and caches, ours added to its prompt;
/// model and effort where named. One process holds the conversation: no
/// one-shot task, no follow-up by id.
// Unix only: unix paths, as the CLI seat has them.
#[cfg(unix)]
#[test]
fn junie_runs_acp_with_every_default_location_off() {
    assert_eq!(
        args_of(&Junie, Some("sonnet"), Some("high")),
        [
            "--acp=true",
            "--skip-update-check",
            "--share-anonymous-statistics=false",
            "--config-default-locations=false",
            "--mcp-default-locations=false",
            "--skill-default-locations=false",
            "--command-default-location=false",
            "--agent-default-location=false",
            "--model-default-locations=false",
            "--extensions-default-location=/s/extensions",
            "--guidelines-filename=/s/guidelines.md",
            "--cache-dir=/s/cache",
            "--system-prompt=SYSTEM",
            "--model=sonnet",
            "--effort=high",
        ]
    );
    let bare = args_of(&Junie, None, None);
    assert!(
        !bare
            .iter()
            .any(|a| a.starts_with("--model=") || a.starts_with("--effort="))
    );
    assert!(!bare.iter().any(|a| a.starts_with("--agent-mode")));
    assert_eq!(
        Junie.files(&settings_for("junie", None), "S"),
        [("guidelines.md", String::new())]
    );
    assert!(!Junie.one_shot() && !Junie.resumes() && !Junie.usage_is_cumulative());
    let content: Value = serde_json::from_str(&Junie.stdin_line("q1\n\"x\"")).unwrap();
    assert_eq!(content, json!([{"type": "text", "text": "q1\n\"x\""}]));
    assert!(Junie.probe_ok(b"Junie version: 26.10.27 (3893.6)\n", b""));
    assert!(!Junie.probe_ok(b"", b""));
}

/// The opening: `initialize` offering no file system and no terminal,
/// `session/new` in the working directory with no MCP server, brave mode
/// asked off (the model and the effort the flags chose are left as they
/// are), and no prompt before Junie confirmed it. Its first message is the
/// start; the session's id is the conversation's, for its files.
#[test]
fn junie_opens_its_session_with_brave_mode_off_before_any_prompt() {
    let (mut drive, greeting) = Drive::new(Some("gemini-3.7-flash"), Some("high"));
    assert_eq!(greeting.len(), 1);
    assert_eq!(greeting[0]["method"], "initialize");
    assert_eq!(greeting[0]["params"]["protocolVersion"], 1);
    assert_eq!(
        greeting[0]["params"]["clientCapabilities"],
        json!({"fs": {"readTextFile": false, "writeTextFile": false}, "terminal": false})
    );
    assert!(drive.send("q1: the board").is_empty(), "held");

    let (events, out) = drive.read(&initialized());
    assert_eq!(events, [Event::Started(Started::default()), Event::Other]);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0]["method"], "session/new");
    assert_eq!(out[0]["params"], json!({"cwd": "/w", "mcpServers": []}));

    // Junie's banner-free noise in between is nothing.
    let auth = json!({"jsonrpc": "2.0", "method": "_auth/status_update",
                      "params": {"authStatus": {"kind": "account"}}});
    assert_eq!(drive.read(&auth), (vec![Event::Other], vec![]));

    let (events, out) = drive.read(&created(&[("model", "gemini-3.7-flash")]));
    assert_eq!(events, [Event::Other]);
    assert_eq!(drive.wire.conversation.as_deref(), Some(SESSION));
    assert_eq!(out.len(), 1, "only brave mode is asked for: {out:?}");
    assert_eq!(out[0]["method"], "session/set_config_option");
    assert_eq!(
        out[0]["params"],
        json!({"sessionId": SESSION, "configId": "brave_mode", "value": "off"})
    );
    let set = out[0]["id"].as_u64().unwrap();

    // Not yet: an update before the confirmation is no answer to it.
    let early = update(&json!({"sessionUpdate": "config_option_update",
                                "configOptions": options(&[("model", "gemini-3.7-flash")])}));
    assert_eq!(drive.read(&early), (vec![Event::Other], vec![]));

    let (events, out) = drive.read(&set_result(
        set,
        &[("brave_mode", "off"), ("model", "gemini-3.7-flash")],
    ));
    assert_eq!(events, [Event::Other]);
    assert_eq!(out.len(), 1, "the held message goes out: {out:?}");
    assert_eq!(out[0]["method"], "session/prompt");
    assert_eq!(
        out[0]["params"],
        json!({"sessionId": SESSION, "prompt": [{"type": "text", "text": "q1: the board"}]})
    );
    let notes = std::mem::take(&mut drive.wire.notes);
    assert_eq!(
        notes,
        [json!({"cli_session": {"tool": "junie", "options": {
            "brave_mode": "off", "model": "gemini-3.7-flash", "effort": "high"}}})]
    );
}

/// A model or an effort the flags did not choose is set where Junie offers
/// it, and left as Junie chose where it does not (an alias); brave mode is
/// asked for whatever Junie says it offers.
#[test]
fn junie_sets_an_offered_model_and_leaves_an_alias() {
    let (mut drive, _) = Drive::new(Some("claude-opus-5-5"), Some("low"));
    drive.read(&initialized());
    let (_, out) = drive.read(&created(&[]));
    let asked: Vec<(&str, &str)> = out
        .iter()
        .map(|line| {
            (
                line["params"]["configId"].as_str().unwrap(),
                line["params"]["value"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        asked,
        [
            ("brave_mode", "off"),
            ("model", "claude-opus-5-5"),
            ("effort", "low")
        ]
    );
    let (mut drive, _) = Drive::new(Some("opus"), None);
    drive.read(&initialized());
    let (_, out) = drive.read(&created(&[("brave_mode", "off")]));
    assert!(out.is_empty(), "nothing to ask for: {out:?}");
    assert_eq!(drive.send("q1").len(), 1, "the opening is through");
}

/// A session that does not confirm brave mode off takes the mind off the
/// table before any message goes out: one whose answer keeps it on, one
/// that refuses to set it, and one that says later it changed.
#[test]
fn junie_without_brave_mode_off_is_a_breach() {
    let open = || {
        let (mut drive, _) = Drive::new(None, None);
        assert!(drive.send("q1").is_empty());
        drive.read(&initialized());
        drive.read(&created(&[]));
        drive
    };
    let (events, out) = open().read(&set_result(3, &[]));
    let [Event::Breach(why)] = events.as_slice() else {
        panic!("{events:?}");
    };
    assert!(why.contains("brave_mode brave-auto, not off"), "{why}");
    assert!(out.is_empty(), "nothing went out");

    let (events, out) = open().read(&failed(3, "Unknown option", &Value::Null));
    let [Event::Breach(why)] = events.as_slice() else {
        panic!("{events:?}");
    };
    assert!(why.contains("would not set its brave_mode"), "{why}");
    assert!(out.is_empty());

    let (mut drive, _) = Drive::opened("q1");
    let flipped = update(&json!({"sessionUpdate": "config_option_update",
        "configOptions": options(&[("brave_mode", "on")])}));
    let (events, _) = drive.read(&flipped);
    assert!(
        matches!(events.as_slice(), [Event::Breach(why)] if why.contains("brave_mode on")),
        "{events:?}"
    );
}

/// A turn as Junie played it: a terminal command asked for, refused by
/// its reject option (once), the call failed, the answer in chunks with
/// prose after it whose braces are no answer, and the turn's own usage.
/// The refusal is counted, not a breach; what the turn cost is a note.
#[test]
fn junie_refuses_every_permission_and_reads_its_reply_in_chunks() {
    let (mut drive, prompt) = Drive::opened("q1");
    let turn = prompt["id"].as_u64().unwrap();
    for line in [
        tool("execute", "pending", "c1", "ls -la ~/.junie"),
        tool("execute", "in_progress", "c1", "ls -la ~/.junie"),
        chunk("Checking the cards. "),
    ] {
        assert_eq!(drive.read(&line), (vec![Event::Other], vec![]));
    }
    let (events, out) = drive.read(&permission(1, "c1", &permission_options()));
    assert_eq!(events, [Event::Refused("execute: ls -la ~/.junie".into())]);
    assert_eq!(
        out,
        [json!({"jsonrpc": "2.0", "id": 1,
                "result": {"outcome": {"outcome": "selected", "optionId": "No"}}})]
    );
    for line in [
        tool("execute", "failed", "c1", "ls -la ~/.junie"),
        update(
            &json!({"sessionUpdate": "usage_update", "used": 14675, "size": 1_048_576,
                       "cost": {"amount": 0.0186, "currency": "USD"}}),
        ),
        chunk("{\"ask\":\"q1\","),
        chunk("\"pick\":[\"a1\"]}\n\n### Changes\n- picked {a1}, kept {mana}"),
        // Another session's words (a sub-agent's) are not the reply.
        update_in(
            "session-other",
            &json!({"sessionUpdate": "agent_message_chunk",
            "content": {"type": "text", "text": "{\"ask\":\"q9\"}"}}),
        ),
    ] {
        assert_eq!(drive.read(&line), (vec![Event::Other], vec![]));
    }
    let (events, out) = drive.read(&done(turn, "end_turn"));
    assert!(out.is_empty());
    let [Event::Reply(Outcome::Answer { value, text, usage })] = events.as_slice() else {
        panic!("{events:?}");
    };
    assert_eq!(value, &Some(json!({"ask": "q1", "pick": ["a1"]})));
    assert!(text.starts_with("Checking the cards. {\"ask\""), "{text}");
    assert_eq!(
        *usage,
        Some(Usage {
            input: 33640,
            output: 1345,
            ..Usage::default()
        })
    );
    assert_eq!(
        std::mem::take(&mut drive.wire.notes),
        [
            json!({"cli_turn": {"tool": "junie", "session_cost": {"amount": 0.0186, "currency": "USD"}}})
        ]
    );
}

/// A permission with no reject option is cancelled; any other request of
/// Junie's is a method the seat does not have, and a file or a terminal of
/// the client's is a tool refused.
#[test]
fn junie_hears_no_to_every_request() {
    let (mut drive, _) = Drive::opened("q1");
    let (events, out) = drive.read(&permission(
        5,
        "c2",
        &json!([{"optionId": "Yes", "kind": "allow_once"}]),
    ));
    assert_eq!(events, [Event::Refused("execute: ls -la ~/.junie".into())]);
    assert_eq!(
        out[0]["result"],
        json!({"outcome": {"outcome": "cancelled"}})
    );
    let asked = json!({"jsonrpc": "2.0", "id": "e1", "method": "session/elicitation",
                       "params": {"sessionId": SESSION}});
    let (events, out) = drive.read(&asked);
    assert_eq!(events, [Event::Other]);
    assert_eq!(out[0]["id"], "e1");
    assert_eq!(out[0]["error"]["code"], -32601);
    let terminal = json!({"jsonrpc": "2.0", "id": 6, "method": "terminal/create",
                          "params": {"sessionId": SESSION, "command": "ls"}});
    let (events, out) = drive.read(&terminal);
    assert_eq!(events, [Event::Refused("terminal/create".into())]);
    assert_eq!(out[0]["error"]["code"], -32601);
}

/// A tool call that shows a tool ran takes the mind off the table: one
/// that completed (refused or not), one that failed without being refused,
/// one still open when its turn ends. The model's own planning is nothing.
#[test]
fn junie_running_a_tool_is_a_breach() {
    let breach = |lines: &[Value]| {
        let (mut drive, prompt) = Drive::opened("q1");
        let turn = prompt["id"].as_u64().unwrap();
        let mut events = Vec::new();
        for line in lines {
            let line = if line.is_null() {
                done(turn, "end_turn")
            } else {
                line.clone()
            };
            events.extend(drive.read(&line).0);
        }
        events.into_iter().find_map(|event| match event {
            Event::Breach(why) => Some(why),
            _ => None,
        })
    };
    let ran = breach(&[
        tool("read", "in_progress", "r1", "/home/u/.junie/x"),
        tool("read", "completed", "r1", "/home/u/.junie/x"),
    ]);
    assert!(
        ran.as_deref()
            .is_some_and(|why| why.contains("(read: /home/u/.junie/x)")
                && why.contains("the seat does not play through it")),
        "{ran:?}"
    );
    let anyway = breach(&[
        tool("execute", "pending", "c1", "ls"),
        permission(1, "c1", &permission_options()),
        tool("execute", "completed", "c1", "ls"),
    ]);
    assert!(anyway.is_some(), "refused, and it ran all the same");
    assert!(breach(&[tool("execute", "failed", "c1", "ls")]).is_some());
    let open = breach(&[tool("fetch", "in_progress", "f1", "https://x"), Value::Null]);
    assert!(open.is_some_and(|why| why.contains("fetch: https://x")));
    assert_eq!(
        breach(&[
            tool("think", "pending", "t1", "plan"),
            tool("think", "completed", "t1", "plan"),
            Value::Null,
        ]),
        None
    );
    assert_eq!(
        breach(&[
            tool("execute", "pending", "c1", "ls"),
            permission(1, "c1", &permission_options()),
            tool("execute", "failed", "c1", "ls"),
            Value::Null,
        ]),
        None,
        "refused, failed: nothing ran"
    );
}

/// One turn at a time: a message sent while a turn runs is held and goes
/// out when it ends.
#[test]
fn junie_holds_a_message_while_a_turn_runs() {
    let (mut drive, prompt) = Drive::opened("q1");
    let turn = prompt["id"].as_u64().unwrap();
    assert!(drive.send("q2").is_empty(), "held");
    drive.read(&chunk("{\"ask\":\"q1\",\"pick\":[\"p\"]}"));
    let (events, out) = drive.read(&done(turn, "end_turn"));
    assert!(matches!(
        events.as_slice(),
        [Event::Reply(Outcome::Answer { .. })]
    ));
    assert_eq!(out.len(), 1);
    assert_eq!(out[0]["params"]["prompt"][0]["text"], "q2");
    drive.read(&chunk("{\"ask\":\"q2\",\"pick\":[\"p\"]}"));
    let next = out[0]["id"].as_u64().unwrap();
    let (events, _) = drive.read(&done(next, "end_turn"));
    let [Event::Reply(Outcome::Answer { value, .. })] = events.as_slice() else {
        panic!("{events:?}");
    };
    assert_eq!(
        value.as_ref().unwrap()["ask"],
        "q2",
        "only its own turn's words"
    );
}

/// A spent balance or a limited provider is a limit; another failure is a
/// failure; an opening that fails ends the process, as unavailable (signed
/// out) or as a limit, never as a breach; a refusal is no answer.
#[test]
fn junie_reads_its_limits_and_failures() {
    let (mut drive, prompt) = Drive::opened("q1");
    let turn = prompt["id"].as_u64().unwrap();
    let spent = failed(
        turn,
        "Insufficient account balance. All tokens in your account have been spent.",
        &json!({"reason": "insufficient_account_balance"}),
    );
    let (events, _) = drive.read(&spent);
    assert!(
        matches!(&events[..], [Event::Reply(Outcome::RateLimited { why, lifts_in: None })]
            if why.starts_with("Insufficient account balance")),
        "{events:?}"
    );
    assert!(!drive.wire.over, "the session goes on");
    let out = drive.send("q2");
    let turn = out[0]["id"].as_u64().unwrap();
    let busy = failed(
        turn,
        "The model provider is temporarily unavailable due to rate limiting. Retry later or \
         switch models.",
        &json!("rate_limit_exceeded"),
    );
    assert!(matches!(
        drive.read(&busy).0[..],
        [Event::Reply(Outcome::RateLimited { .. })]
    ));
    let turn = drive.send("q3")[0]["id"].as_u64().unwrap();
    let (events, _) = drive.read(&failed(turn, "Internal error", &Value::Null));
    assert_eq!(
        events,
        [Event::Reply(Outcome::Failed("Internal error".into()))]
    );
    let turn = drive.send("q4")[0]["id"].as_u64().unwrap();
    drive.read(&chunk("I won't."));
    let (events, _) = drive.read(&done(turn, "refusal"));
    assert!(
        matches!(&events[..], [Event::Reply(Outcome::Failed(why))] if why.contains("refusal")),
        "{events:?}"
    );

    let (mut drive, _) = Drive::new(None, None);
    drive.read(&initialized());
    let signed_out = failed(2, "Authentication required", &Value::Null);
    let (events, _) = drive.read(&signed_out);
    assert!(
        matches!(&events[..], [Event::Reply(Outcome::Failed(why))]
            if why.starts_with("junie is not signed in (Authentication required)")),
        "{events:?}"
    );
    assert!(drive.wire.over, "the process is done with");
    assert!(drive.send("q1").is_empty(), "nothing goes to it");

    let (mut drive, _) = Drive::new(None, None);
    let (events, _) = drive.read(&failed(1, "Insufficient account balance", &Value::Null));
    assert!(
        matches!(
            &events[..],
            [Event::Started(_), Event::Reply(Outcome::RateLimited { .. })]
        ),
        "a limit as its first word is a limit after its start: {events:?}"
    );
    assert!(drive.wire.over);
}

/// A turn's tokens are booked so their whole is Junie's own total, whether
/// its input counts the cached ones or not; a result without usage is
/// booked at its worst.
#[test]
fn junie_books_a_turn_by_its_own_total() {
    let reply = |usage: Value| {
        let (mut drive, prompt) = Drive::opened("q1");
        let turn = prompt["id"].as_u64().unwrap();
        drive.read(&chunk("{\"ask\":\"q1\"}"));
        let line = json!({"jsonrpc": "2.0", "id": turn,
                          "result": {"stopReason": "end_turn", "usage": usage}});
        let (events, _) = drive.read(&line);
        let [Event::Reply(Outcome::Answer { usage, .. })] = events.as_slice() else {
            panic!("{events:?}");
        };
        *usage
    };
    let inside = reply(
        json!({"inputTokens": 1100, "outputTokens": 20, "totalTokens": 1120,
                              "cachedReadTokens": 1000, "cachedWriteTokens": 0}),
    )
    .unwrap();
    assert_eq!(
        (inside.input, inside.cache_read, inside.total()),
        (100, 1000, 1120)
    );
    let beside = reply(
        json!({"inputTokens": 100, "outputTokens": 20, "totalTokens": 1150,
                              "cachedReadTokens": 1000, "cachedWriteTokens": 30}),
    )
    .unwrap();
    assert_eq!(
        (beside.input, beside.cache_write, beside.total()),
        (100, 30, 1150)
    );
    let unsaid = reply(json!({"inputTokens": 7, "outputTokens": 2})).unwrap();
    assert_eq!((unsaid.input, unsaid.output), (7, 2));
    assert_eq!(reply(json!({})), None);
}

/// Junie's sessions are where its home is (`JUNIE_HOME`, else `~/.junie`),
/// and the seat's own is exactly the directory named for its id: never a
/// link, a file, another session's or an id that is not one.
#[cfg(unix)]
#[test]
fn junie_names_exactly_its_sessions_directory() {
    let env = |name: &str| (name == "HOME").then(|| OsString::from("/home/t"));
    assert_eq!(
        Junie.sessions_root(&env),
        Some(PathBuf::from("/home/t/.junie/sessions"))
    );
    let moved = |name: &str| match name {
        "HOME" => Some(OsString::from("/home/t")),
        "JUNIE_HOME" => Some(OsString::from("/j")),
        _ => None,
    };
    assert_eq!(
        Junie.sessions_root(&moved),
        Some(PathBuf::from("/j/sessions"))
    );
    let root = scratch_dir("junie-files");
    let ours = root.join("session-ours");
    let other = root.join("session-other");
    std::fs::create_dir_all(&ours).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(root.join("session-file"), "").unwrap();
    std::os::unix::fs::symlink(&other, root.join("session-link")).unwrap();
    assert_eq!(Junie.session_files(&root, "session-ours"), [ours]);
    for none in [
        "session-file",
        "session-link",
        "session-gone",
        "..",
        "-x",
        "",
    ] {
        assert!(Junie.session_files(&root, none).is_empty(), "{none:?}");
    }
    std::fs::remove_dir_all(&root).unwrap();
}
