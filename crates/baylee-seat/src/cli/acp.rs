//! The Agent Client Protocol (ACP) as the seat speaks it, as the client of
//! an agent CLI that holds one session for the whole game: JSON-RPC 2.0,
//! one JSON object a line, both ways.
//!
//! The seat offers the agent nothing a client may offer: no file system
//! (`fs.readTextFile`, `fs.writeTextFile` false), no terminal, no MCP
//! server. It opens one session in the process's empty working directory,
//! asks for the session options its dialect names ([`Want`]: Junie's brave
//! mode off) and sends no message before the agent confirmed each one that
//! must hold. Every request of the agent's is answered at once, so a turn
//! never waits on the seat: a permission is always refused (the reject
//! option the agent offers, once where it offers that, else `cancelled`),
//! and anything else is a method the seat does not have.
//!
//! A refused permission is no breach: nothing ran, and the turn goes on
//! ([`Event::Refused`]). A tool call that shows a tool did run (one that
//! completed, one that failed without being refused, one still open when
//! its turn ends) takes the mind off the table ([`Event::Breach`]), as
//! does a session option that must hold and does not, at the opening or at
//! any time after. Only a call of the `think` kind (the model's own
//! planning, which touches nothing) is left alone.
//!
//! One turn at a time: a message sent while a turn runs (or before the
//! session is open) is held, and sent when the turn before it ends. A
//! turn's reply is the text of its `agent_message_chunk`s, its usage the
//! prompt result's. The agent's first message, whatever it says, is the
//! process's start ([`Event::Started`]): nothing is sent to the model
//! before the opening is through, so no tool can run before it. An opening
//! that fails is a reply (unavailable, or a limit), and the process is
//! done with ([`Wire::over`]).

use super::dialect::{Event, Outcome, Started, Wire, clipped, conversation_id};
use crate::llm::{Usage, json_object};
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};
use std::path::Path;

/// The protocol version the seat speaks.
const PROTOCOL_VERSION: u64 = 1;

/// JSON-RPC's code for a method the receiver does not have.
const METHOD_NOT_FOUND: i64 = -32601;

/// The tool-call kind that touches nothing: the model's own planning.
const THINK: &str = "think";

/// The agent the seat talks to.
pub(crate) struct Agent {
    /// The tool's name, for a sentence.
    pub(crate) name: &'static str,
    /// Whether a failure's words say it is a rate limit or a spent quota.
    pub(crate) limited: fn(&str) -> bool,
    /// Whether a failure's words say the tool is not signed in.
    pub(crate) signed_out: fn(&str) -> bool,
}

/// A session option the seat asks for before it plays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Want {
    /// The option's id (`brave_mode`).
    pub(crate) id: &'static str,
    /// The value asked for.
    pub(crate) value: String,
    /// Whether the session must hold it, confirmed by the agent, or the
    /// seat does not play through it. One that need not is set where the
    /// agent offers the value and left as the agent chose otherwise (a
    /// model a flag named by an alias, say).
    pub(crate) must: bool,
}

/// One process's side of the protocol, kept in its [`Wire`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Acp {
    stage: Stage,
    /// The id of the seat's next request.
    next: u64,
    /// The seat's requests not answered yet, by id.
    asked: BTreeMap<u64, Asked>,
    /// The working directory the session is opened in.
    cwd: String,
    /// The options asked for at the opening.
    wants: Vec<Want>,
    /// Each option as the agent last said it, by id.
    options: BTreeMap<String, Choice>,
    /// The session's id, once opened.
    session: Option<String>,
    /// Messages not sent yet: the session is not open, or a turn runs.
    held: VecDeque<String>,
    /// The turn running: its prompt's id.
    turn: Option<u64>,
    /// The tool calls of the turn running, by id.
    calls: BTreeMap<String, Call>,
    /// What the agent last said the whole session has cost so far
    /// (`usage_update`, a running total), as it said it: noted at the end
    /// of each turn.
    cost: Option<Value>,
}

/// How far the session is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    /// `initialize` sent.
    #[default]
    Greeting,
    /// `session/new` sent.
    Opening,
    /// The options asked for, not all answered.
    Setting,
    /// Open: messages go out.
    Ready,
    /// The opening failed: nothing more goes out.
    Broken,
}

/// What a request of the seat's asked.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Asked {
    Initialize,
    NewSession,
    Set(&'static str),
    Prompt,
}

/// A session option as the agent said it: its value now, and the values
/// it offers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Choice {
    current: Option<String>,
    offered: Vec<String>,
}

/// A tool call of the turn, as far as the agent said.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Call {
    kind: String,
    title: String,
    /// The seat refused its permission.
    refused: bool,
    /// It came to an end (`completed`, `failed`).
    ended: bool,
}

impl Call {
    /// What it was, for a sentence.
    fn said(&self) -> String {
        let kind = if self.kind.is_empty() {
            "a tool"
        } else {
            &self.kind
        };
        if self.title.is_empty() {
            kind.to_string()
        } else {
            format!("{kind}: {}", clipped(&self.title))
        }
    }
}

/// Opens the protocol on `wire`: the session will be opened in `cwd` with
/// `wants`. The line that greets the agent.
pub(crate) fn open(wire: &mut Wire, cwd: &Path, wants: Vec<Want>) -> Vec<String> {
    wire.acp = Acp {
        cwd: cwd.to_string_lossy().into_owned(),
        wants,
        next: 1,
        ..Acp::default()
    };
    let capabilities = json!({
        "fs": {"readTextFile": false, "writeTextFile": false},
        "terminal": false,
    });
    vec![request(
        &mut wire.acp,
        Asked::Initialize,
        "initialize",
        &json!({"protocolVersion": PROTOCOL_VERSION, "clientCapabilities": capabilities}),
    )]
}

/// The content of a prompt that says `text`.
pub(crate) fn content(text: &str) -> Value {
    json!([{"type": "text", "text": text}])
}

/// The lines that send `text` as the session's next message: its prompt,
/// or nothing while it is held.
pub(crate) fn message(wire: &mut Wire, text: &str) -> Vec<String> {
    wire.acp.held.push_back(text.to_string());
    next_prompt(wire).into_iter().collect()
}

/// Reads one line of the agent's.
pub(crate) fn read_event(line: &str, wire: &mut Wire, agent: &Agent) -> Event {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return Event::Other;
    };
    if value.get("jsonrpc").is_none() {
        return Event::Other;
    }
    // Its first message is its start, whatever it says; then it is read
    // again for itself.
    if wire.session.is_none() {
        wire.session = Some(String::new());
        wire.again = true;
        return Event::Started(Started::default());
    }
    let method = value.get("method").and_then(Value::as_str);
    match (value.get("id"), method) {
        (Some(id), Some(method)) => answer_request(id, method, &value, wire),
        (None, Some(method)) => notification(method, &value, wire, agent),
        (Some(id), None) => id
            .as_u64()
            .map_or(Event::Other, |id| response(id, &value, wire, agent)),
        (None, None) => Event::Other,
    }
}

/// A request of the seat's, as a line, noted as `asked`.
fn request(acp: &mut Acp, asked: Asked, method: &str, params: &Value) -> String {
    let id = acp.next;
    acp.next += 1;
    acp.asked.insert(id, asked);
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string()
}

/// The prompt of the next message held, where the session is open and no
/// turn runs.
fn next_prompt(wire: &mut Wire) -> Option<String> {
    let acp = &mut wire.acp;
    if acp.stage != Stage::Ready || acp.turn.is_some() {
        return None;
    }
    let text = acp.held.pop_front()?;
    let session = acp.session.clone().unwrap_or_default();
    acp.turn = Some(acp.next);
    acp.calls.clear();
    let line = request(
        acp,
        Asked::Prompt,
        "session/prompt",
        &json!({"sessionId": session, "prompt": content(&text)}),
    );
    wire.said = None;
    Some(line)
}

/// Answers a request of the agent's at once: a permission refused, any
/// other method not had.
fn answer_request(id: &Value, method: &str, value: &Value, wire: &mut Wire) -> Event {
    let params = value.get("params").unwrap_or(&Value::Null);
    if method == "session/request_permission" {
        let call = params.get("toolCall").unwrap_or(&Value::Null);
        let refused = remember(&mut wire.acp, call, true);
        let outcome = params
            .get("options")
            .and_then(Value::as_array)
            .and_then(|options| reject_option(options))
            .map_or_else(
                || json!({"outcome": "cancelled"}),
                |option| json!({"outcome": "selected", "optionId": option}),
            );
        wire.out
            .push(json!({"jsonrpc": "2.0", "id": id, "result": {"outcome": outcome}}).to_string());
        return Event::Refused(refused);
    }
    wire.out.push(
        json!({"jsonrpc": "2.0", "id": id, "error": {
            "code": METHOD_NOT_FOUND,
            "message": format!("the seat offers no {method}"),
        }})
        .to_string(),
    );
    // A file or a terminal of the client's is a tool too, refused as one.
    if method.starts_with("fs/") || method.starts_with("terminal/") {
        Event::Refused(clipped(method))
    } else {
        Event::Other
    }
}

/// The option that refuses a permission: once where the agent offers that
/// (so it keeps nothing for later), else always.
fn reject_option(options: &[Value]) -> Option<Value> {
    ["reject_once", "reject_always"].iter().find_map(|kind| {
        options
            .iter()
            .find(|option| option.get("kind").and_then(Value::as_str) == Some(kind))
            .and_then(|option| option.get("optionId"))
            .cloned()
    })
}

/// Notes a tool call's fields as the agent sent them (`refused` where the
/// seat refuses it now); what it is, for a sentence.
fn remember(acp: &mut Acp, call: &Value, refused: bool) -> String {
    let text = |key: &str| call.get(key).and_then(Value::as_str);
    let id = text("toolCallId").unwrap_or_default().to_string();
    let entry = acp.calls.entry(id).or_default();
    if let Some(kind) = text("kind") {
        entry.kind = kind.to_string();
    }
    if let Some(title) = text("title") {
        entry.title = title.to_string();
    }
    entry.refused |= refused;
    entry.said()
}

/// A notification of the agent's.
fn notification(method: &str, value: &Value, wire: &mut Wire, agent: &Agent) -> Event {
    if method != "session/update" {
        return Event::Other;
    }
    let params = value.get("params").unwrap_or(&Value::Null);
    let update = params.get("update").unwrap_or(&Value::Null);
    let ours = wire.acp.session.is_some()
        && params.get("sessionId").and_then(Value::as_str) == wire.acp.session.as_deref();
    match update.get("sessionUpdate").and_then(Value::as_str) {
        Some("agent_message_chunk") if ours && wire.acp.turn.is_some() => {
            let content = update.get("content").unwrap_or(&Value::Null);
            if content.get("type").and_then(Value::as_str) == Some("text")
                && let Some(text) = content.get("text").and_then(Value::as_str)
            {
                wire.said.get_or_insert_with(String::new).push_str(text);
            }
            Event::Other
        }
        // Any session's: a sub-agent's call runs on this machine too.
        Some("tool_call" | "tool_call_update") => tool_call(update, wire, agent),
        Some("usage_update") if ours => {
            if let Some(cost) = update.get("cost") {
                wire.acp.cost = Some(cost.clone());
            }
            Event::Other
        }
        Some("config_option_update") if ours => {
            note_options(&mut wire.acp, update.get("configOptions"));
            match unmet(&wire.acp, agent) {
                Some((_, why)) if wire.acp.stage == Stage::Ready => Event::Breach(why),
                _ => Event::Other,
            }
        }
        _ => Event::Other,
    }
}

/// A tool call's news: one that shows a tool ran is a breach.
fn tool_call(update: &Value, wire: &mut Wire, agent: &Agent) -> Event {
    let said = remember(&mut wire.acp, update, false);
    let id = update
        .get("toolCallId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let Some(call) = wire.acp.calls.get_mut(id) else {
        return Event::Other;
    };
    let status = update.get("status").and_then(Value::as_str);
    if matches!(status, Some("completed" | "failed")) {
        call.ended = true;
    }
    let ran = call.kind != THINK
        && match status {
            Some("completed") => true,
            Some("failed") => !call.refused,
            _ => false,
        };
    if ran {
        Event::Breach(ran_a_tool(agent, &said))
    } else {
        Event::Other
    }
}

/// Why a tool that ran takes the mind off the table.
fn ran_a_tool(agent: &Agent, said: &str) -> String {
    format!(
        "the {} process let the model use a tool ({said}): the seat does not play through it",
        agent.name
    )
}

/// A response to a request of the seat's.
fn response(id: u64, value: &Value, wire: &mut Wire, agent: &Agent) -> Event {
    let Some(asked) = wire.acp.asked.remove(&id) else {
        return Event::Other;
    };
    let failure = value.get("error").map(error_words);
    let result = value.get("result").unwrap_or(&Value::Null);
    if asked == Asked::Prompt {
        return turn_end(failure, result, wire, agent);
    }
    if wire.acp.stage == Stage::Broken {
        return Event::Other;
    }
    match (asked, failure) {
        (Asked::Set(option), Some(why)) => {
            let must = wire
                .acp
                .wants
                .iter()
                .any(|want| want.id == option && want.must);
            if must {
                return Event::Breach(format!(
                    "the {} process would not set its {option} ({}): the seat does not play \
                     through it",
                    agent.name,
                    clipped(&why)
                ));
            }
            opened(wire, agent)
        }
        (_, Some(why)) => broken(&why, wire, agent),
        (Asked::Initialize, None) => {
            let version = result.get("protocolVersion").and_then(Value::as_u64);
            if let Some(version) = version.filter(|version| *version != PROTOCOL_VERSION) {
                return broken(&format!("it speaks ACP version {version}"), wire, agent);
            }
            wire.acp.stage = Stage::Opening;
            let params = json!({"cwd": wire.acp.cwd, "mcpServers": []});
            let line = request(&mut wire.acp, Asked::NewSession, "session/new", &params);
            wire.out.push(line);
            Event::Other
        }
        (Asked::NewSession, None) => {
            let Some(session) = result.get("sessionId").and_then(Value::as_str) else {
                return broken("it opened no session", wire, agent);
            };
            wire.acp.session = Some(session.to_string());
            wire.conversation = conversation_id(session);
            note_options(&mut wire.acp, result.get("configOptions"));
            wire.acp.stage = Stage::Setting;
            ask_options(wire);
            opened(wire, agent)
        }
        (Asked::Set(_), None) => {
            note_options(&mut wire.acp, result.get("configOptions"));
            opened(wire, agent)
        }
        (Asked::Prompt, None) => Event::Other,
    }
}

/// Asks for each option wanted that the session does not have yet: one
/// that must hold whatever the agent offers, another where it offers the
/// value.
fn ask_options(wire: &mut Wire) {
    let acp = &mut wire.acp;
    let session = acp.session.clone().unwrap_or_default();
    let asking: Vec<Want> = acp
        .wants
        .iter()
        .filter_map(|want| {
            let choice = acp.options.get(want.id);
            if holds(want, choice.and_then(|choice| choice.current.as_deref())) {
                return None;
            }
            match choice.and_then(|choice| offered(want, choice)) {
                Some(value) => Some(Want {
                    value,
                    ..want.clone()
                }),
                // One the agent does not offer is asked for all the same
                // where it must hold; the answer says whether it does.
                None => want.must.then(|| want.clone()),
            }
        })
        .collect();
    for want in asking {
        let params = json!({"sessionId": session, "configId": want.id, "value": want.value});
        let line = request(
            acp,
            Asked::Set(want.id),
            "session/set_config_option",
            &params,
        );
        wire.out.push(line);
    }
}

/// Keeps each option the agent named: its value now and the values it
/// offers (grouped or not).
fn note_options(acp: &mut Acp, options: Option<&Value>) {
    fn values(options: &[Value], into: &mut Vec<String>) {
        for option in options {
            if let Some(value) = option.get("value").and_then(Value::as_str) {
                into.push(value.to_string());
            }
            if let Some(inner) = option.get("options").and_then(Value::as_array) {
                values(inner, into);
            }
        }
    }
    for option in options.and_then(Value::as_array).into_iter().flatten() {
        let Some(id) = option.get("id").and_then(Value::as_str) else {
            continue;
        };
        let mut offered = Vec::new();
        if let Some(options) = option.get("options").and_then(Value::as_array) {
            values(options, &mut offered);
        }
        let current = option
            .get("currentValue")
            .and_then(Value::as_str)
            .map(str::to_string);
        acp.options
            .insert(id.to_string(), Choice { current, offered });
    }
}

/// Whether `current` is the value `want` asks for: that value, or an
/// agent's full id of it, whose last `:` part it is (Junie's model
/// `v1:12:jetbrains-ai:claude-opus-5-5` for `claude-opus-5-5`).
fn holds(want: &Want, current: Option<&str>) -> bool {
    current.is_some_and(|current| {
        current == want.value || current.rsplit(':').next() == Some(want.value.as_str())
    })
}

/// The value the agent offers for `want`: the very value, else the one
/// whose full id it names ([`holds`]).
fn offered(want: &Want, choice: &Choice) -> Option<String> {
    choice
        .offered
        .iter()
        .find(|value| **value == want.value)
        .or_else(|| {
            choice
                .offered
                .iter()
                .find(|value| holds(want, Some(value.as_str())))
        })
        .cloned()
}

/// The option the session does not hold though it must, and why: `None`
/// where it holds every one.
fn unmet(acp: &Acp, agent: &Agent) -> Option<(&'static str, String)> {
    acp.wants.iter().filter(|want| want.must).find_map(|want| {
        let current = acp
            .options
            .get(want.id)
            .and_then(|choice| choice.current.as_deref());
        (!holds(want, current)).then(|| {
            (
                want.id,
                format!(
                    "the {} process's session has {} {}, not {}: the seat does not play through it",
                    agent.name,
                    want.id,
                    current.unwrap_or("unsaid"),
                    want.value
                ),
            )
        })
    })
}

/// The opening, once every option asked for is answered: the session
/// holds what it must, and the messages held go out. A breach where it
/// does not.
fn opened(wire: &mut Wire, agent: &Agent) -> Event {
    let acp = &mut wire.acp;
    let setting = acp
        .asked
        .values()
        .any(|asked| matches!(asked, Asked::Set(_)));
    if acp.stage != Stage::Setting || setting {
        return Event::Other;
    }
    match unmet(acp, agent) {
        // A model the agent does not offer is one this session cannot
        // play, nothing it did: the model is unavailable, and a fallback
        // may take the game, never another model under its name.
        Some(("model", why)) => return broken(&why, wire, agent),
        Some((_, why)) => return Event::Breach(why),
        None => {}
    }
    let acp = &mut wire.acp;
    acp.stage = Stage::Ready;
    let chosen: BTreeMap<&str, Option<&str>> = acp
        .wants
        .iter()
        .map(|want| {
            let current = acp
                .options
                .get(want.id)
                .and_then(|choice| choice.current.as_deref());
            (want.id, current)
        })
        .collect();
    let note = json!({"cli_session": {"tool": agent.name, "options": chosen}});
    wire.notes.push(note);
    if let Some(line) = next_prompt(wire) {
        wire.out.push(line);
    }
    Event::Other
}

/// An opening that failed for `why`: the process is done with, and the
/// question waiting hears it (a limit, or unavailable).
fn broken(why: &str, wire: &mut Wire, agent: &Agent) -> Event {
    wire.acp.stage = Stage::Broken;
    wire.acp.held.clear();
    wire.over = true;
    Event::Reply(failed(why, agent))
}

/// A failure's outcome by its words: a limit, a sign-in missing, or else.
fn failed(why: &str, agent: &Agent) -> Outcome {
    if (agent.limited)(why) {
        Outcome::RateLimited {
            why: clipped(why),
            lifts_in: None,
        }
    } else if (agent.signed_out)(why) {
        Outcome::Failed(format!(
            "{} is not signed in ({}): sign in once with {} itself",
            agent.name,
            clipped(why),
            agent.name
        ))
    } else {
        Outcome::Failed(clipped(why))
    }
}

/// The end of a turn: its reply, or why there is none; a breach where a
/// tool call of the turn is still open and was not refused. The next
/// message held goes out.
fn turn_end(failure: Option<String>, result: &Value, wire: &mut Wire, agent: &Agent) -> Event {
    let said = wire.said.take().unwrap_or_default();
    let acp = &mut wire.acp;
    acp.turn = None;
    let open = acp
        .calls
        .values()
        .find(|call| !call.ended && !call.refused && call.kind != THINK)
        .map(Call::said);
    if let Some(open) = open {
        return Event::Breach(ran_a_tool(agent, &open));
    }
    if let Some(cost) = acp.cost.take() {
        wire.notes
            .push(json!({"cli_turn": {"tool": agent.name, "session_cost": cost}}));
    }
    if let Some(line) = next_prompt(wire) {
        wire.out.push(line);
    }
    if let Some(why) = failure {
        return Event::Reply(failed(&why, agent));
    }
    let stop = result
        .get("stopReason")
        .and_then(Value::as_str)
        .unwrap_or("end_turn");
    let value = json_object(&said);
    if !matches!(stop, "end_turn" | "max_tokens") {
        return Event::Reply(Outcome::Failed(format!(
            "the {} turn ended without an answer ({stop})",
            agent.name
        )));
    }
    // A limit the agent says as its text, with no answer in it.
    if value.is_none() && (agent.limited)(&said) {
        return Event::Reply(failed(&said, agent));
    }
    Event::Reply(Outcome::Answer {
        value,
        text: said,
        usage: result.get("usage").and_then(usage),
    })
}

/// A JSON-RPC error in words: its message, and what its data says.
fn error_words(error: &Value) -> String {
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("an error");
    match error.get("data") {
        Some(Value::String(data)) => format!("{message} ({data})"),
        Some(data @ Value::Object(_)) => format!("{message} ({data})"),
        _ => message.to_string(),
    }
}

/// A turn's tokens, as the prompt result counts them, booked so that the
/// whole is the agent's own `totalTokens` where it says one: whether its
/// `inputTokens` holds the cached ones or not, what is neither output nor
/// cached is input.
fn usage(usage: &Value) -> Option<Usage> {
    let n = |key: &str| usage.get(key).and_then(Value::as_u64);
    let input = n("inputTokens")?;
    let output = n("outputTokens").unwrap_or(0) + n("thoughtTokens").unwrap_or(0);
    let cache_read = n("cachedReadTokens").unwrap_or(0);
    let cache_write = n("cachedWriteTokens").unwrap_or(0);
    let input = n("totalTokens").map_or(input, |total| {
        total.saturating_sub(output + cache_read + cache_write)
    });
    Some(Usage {
        input,
        output,
        cache_write,
        cache_read,
        ..Usage::default()
    })
}
