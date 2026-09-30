//! End-to-end: a language-model seat plays a whole game through a real
//! gateway, against a stand-in provider on this machine.
//!
//! The seat is `baylee-seat`'s `ApiMind` behind the bridge, seated as
//! `baylee-seat join --mind anthropic:claude-sonnet-5` seats it; only the
//! provider is a stand-in. It reads every request as the Messages API
//! would and refuses one that breaks the API's rules (roles alternate, a
//! result for every call, the model's own turns replayed unchanged,
//! thinking and all), then answers the question the narrator wrote with a
//! plain policy: play a land, cast what it can, attack with everything,
//! never block. Once in the game it answers with an id that is on no list,
//! and the mind has to send that back and take the second answer. No
//! network and no key: the key is a placeholder shaped like one, so the
//! scrubber's patterns are held to it too.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use baylee_seat::bridge::{self, PlayOptions};
use baylee_seat::deck::Deck;
use baylee_seat::link::SeatLink;
use baylee_seat::llm::{ApiMind, Provider, Settings, Spec, credentials};
use baylee_seat::lobby::{GuestSignIn, Lobby, Session, seat_name};
use baylee_seat::seat::Outcome;
use baylee_seat::{BridgeConfig, HouseMind, Mind, SeatCore, Transcript};
use common::{attach_agent, spawn_gateway};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// A whole game, with room to spare.
const GAME_BUDGET: Duration = Duration::from_secs(600);

/// Shaped like an Anthropic key so the scrubber would know it; not a key
/// anywhere.
const KEY: &str = "sk-ant-TEST-0123456789abcdefghijklmn";

/// The call the stand-in answers with an id that is on no list.
const SPOILED_CALL: u64 = 3;

/// What the stand-in saw and said.
#[derive(Default)]
struct Books {
    calls: u64,
    /// Every content the stand-in answered with: an assistant turn in a
    /// later request must be one of these, unchanged.
    said: Vec<Value>,
    /// Requests that broke the API's rules, and why.
    faults: Vec<String>,
    /// The id of the call answered with an id on no list.
    spoiled: Option<String>,
    /// The next request carried that call's error back.
    sent_back: bool,
    /// Questions answered, by the shape of their answer.
    shapes: BTreeMap<&'static str, u32>,
}

type Shared = Arc<Mutex<Books>>;

fn lock(books: &Shared) -> MutexGuard<'_, Books> {
    books.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A stand-in provider on a free loopback port.
async fn stand_in() -> (String, Shared) {
    let books = Shared::default();
    let app = Router::new()
        .route("/v1/messages", post(messages))
        // Where a mind that is down asks whether it may come back.
        .route(
            "/v1/models/{model}",
            get(|| async { axum::Json(json!({"type": "model", "id": "claude-sonnet-5"})) }),
        )
        .fallback(elsewhere)
        .with_state(books.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a port");
    let addr = listener.local_addr().expect("an address");
    tokio::spawn(async move { axum::serve(listener, app).await });
    (format!("http://{addr}"), books)
}

async fn elsewhere(State(books): State<Shared>, uri: Uri) -> StatusCode {
    lock(&books)
        .faults
        .push(format!("a request to {}", uri.path()));
    StatusCode::NOT_FOUND
}

async fn messages(State(books): State<Shared>, headers: HeaderMap, body: Bytes) -> Response {
    let mut books = lock(&books);
    books.calls += 1;
    let call = books.calls;
    let raw = String::from_utf8_lossy(&body);
    let request: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let faults = check(&headers, &raw, &request, &books.said);
    if !faults.is_empty() {
        let message = faults.join("; ");
        books
            .faults
            .extend(faults.into_iter().map(|f| format!("call {call}: {f}")));
        let body = json!({"type": "error", "error": {"type": "invalid_request_error", "message": message}});
        return (StatusCode::BAD_REQUEST, axum::Json(body)).into_response();
    }
    if let Some(spoiled) = books.spoiled.clone()
        && sends_back(&request, &spoiled)
    {
        books.sent_back = true;
    }

    let question = latest_question(&request);
    let (shape, mut input) = decide(&question);
    *books.shapes.entry(shape).or_default() += 1;
    let id = format!("toolu_TEST{call}");
    if call == SPOILED_CALL {
        input["pick"] = json!(["zz9"]);
        books.spoiled = Some(id.clone());
    }
    let content = json!([
        {"type": "thinking", "thinking": format!("Call {call}: the plain answer."),
         "signature": format!("TEST-sig-{call}")},
        {"type": "tool_use", "id": id, "name": "decide", "input": input},
    ]);
    books.said.push(content.clone());
    axum::Json(json!({
        "id": format!("msg_TEST{call}"), "type": "message", "role": "assistant",
        "model": request["model"], "content": content, "stop_reason": "tool_use",
        "usage": {"input_tokens": 400, "output_tokens": 150,
                  "cache_creation_input_tokens": 0, "cache_read_input_tokens": 2000}
    }))
    .into_response()
}

/// What in a request the Messages API would refuse, or this test would.
fn check(headers: &HeaderMap, raw: &str, request: &Value, said: &[Value]) -> Vec<String> {
    let mut faults = Vec::new();
    let mut fault = |f: String| faults.push(f);
    let header = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());
    if header("x-api-key") != Some(KEY) {
        fault("no key, or another one, in x-api-key".into());
    }
    if header("anthropic-version").is_none() {
        fault("no anthropic-version".into());
    }
    if raw.contains(KEY) {
        fault("the key is in the body".into());
    }
    if request["model"] != "claude-sonnet-5" || request["max_tokens"].as_u64().is_none() {
        fault(format!(
            "model {} / max_tokens {}",
            request["model"], request["max_tokens"]
        ));
    }
    if request["tool_choice"]["type"] != "auto" || request["thinking"]["type"] != "adaptive" {
        fault("tool_choice auto and adaptive thinking".into());
    }
    let system_cached = request["system"]
        .as_array()
        .and_then(|s| s.last())
        .is_some_and(|b| b.get("cache_control").is_some());
    if !system_cached || request.get("cache_control").is_none() {
        fault("the system prompt and the conversation are not cached".into());
    }
    let Some(messages) = request["messages"].as_array() else {
        fault("no messages".into());
        return faults;
    };
    if messages.last().is_none_or(|m| m["role"] != "user") {
        fault("the last message is not the user's".into());
    }
    for (i, message) in messages.iter().enumerate() {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        if message["role"] != role {
            fault(format!("message {i} is {}, not {role}", message["role"]));
            continue;
        }
        if role == "user" {
            continue;
        }
        // The model's own turn goes back as it came: a thinking block
        // whose signature no longer matches is refused.
        if !said.contains(&message["content"]) {
            fault(format!("assistant turn {i} is not what the model said"));
        }
        let calls: Vec<&Value> = blocks(message, "tool_use")
            .into_iter()
            .map(|b| &b["id"])
            .collect();
        let next = messages.get(i + 1).and_then(|m| m["content"].as_array());
        let results: Vec<&Value> = next
            .into_iter()
            .flatten()
            .take_while(|b| b["type"] == "tool_result")
            .map(|b| &b["tool_use_id"])
            .collect();
        if calls.len() != results.len() || !calls.iter().all(|c| results.contains(c)) {
            fault(format!(
                "turn {i} called {calls:?}; the next message answers {results:?} first"
            ));
        }
    }
    faults
}

fn blocks<'a>(message: &'a Value, kind: &str) -> Vec<&'a Value> {
    message["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|b| b["type"] == kind)
        .collect()
}

/// Whether the last user message sends back the spoiled call as an error.
fn sends_back(request: &Value, spoiled: &str) -> bool {
    request["messages"]
        .as_array()
        .and_then(|m| m.last())
        .is_some_and(|m| {
            blocks(m, "tool_result")
                .iter()
                .any(|b| b["tool_use_id"] == spoiled && b["is_error"] == true)
        })
}

/// The latest question the narrator wrote, from its `QUESTION:` line on.
fn latest_question(request: &Value) -> String {
    let mut text = String::new();
    for message in request["messages"].as_array().into_iter().flatten() {
        if message["role"] == "user" {
            for block in blocks(message, "text") {
                text.push_str(block["text"].as_str().unwrap_or_default());
                text.push('\n');
            }
        }
    }
    text.rfind("QUESTION:")
        .map_or_else(String::new, |at| text[at..].to_string())
}

/// The stand-in's answer: the shape it read, and the `decide` input.
fn decide(question: &str) -> (&'static str, Value) {
    let line = question
        .lines()
        .rev()
        .find(|l| l.starts_with("Answer with decide: ask=\""))
        .unwrap_or_default();
    let ask = line.split('"').nth(1).unwrap_or_default();
    let shape = line.split_once(", ").map_or("", |(_, s)| s);
    let mut input = json!({"ask": ask});
    let kind = if shape.starts_with("attacks=") {
        input["attacks"] = attacks(question);
        input["say"] = json!("Everything goes in.");
        "attack"
    } else if shape.starts_with("blocks=") {
        input["blocks"] = json!([]);
        "block"
    } else if let Some(range) = shape.strip_prefix("number=<an integer from ") {
        let least: i64 = range
            .split(' ')
            .next()
            .and_then(|n| n.parse().ok())
            .unwrap_or(0);
        input["number"] = json!(least);
        "number"
    } else if shape.starts_with("piles=") {
        input["piles"] = piles(question);
        "piles"
    } else if shape.starts_with("name=\"<the type>") {
        input["name"] = json!(a_type(question));
        "type"
    } else if shape.starts_with("name=") {
        input["name"] = json!("Lightning Bolt");
        "card name"
    } else if shape.starts_with("pick=[one option id]") {
        input["pick"] = json!([option(question)]);
        "option"
    } else {
        input["pick"] = json!(pick(question, shape));
        "pick"
    };
    (kind, input)
}

/// The list right above the answer line: `(id, label)` per line.
fn last_list(question: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = question.lines().collect();
    let end = lines
        .iter()
        .rposition(|l| l.starts_with("Answer with decide"))
        .unwrap_or(lines.len());
    let start = lines[..end]
        .iter()
        .rposition(|l| !l.starts_with("  "))
        .map_or(0, |i| i + 1);
    lines[start..end].iter().filter_map(|l| item(l)).collect()
}

/// The list under `header`.
fn list_under(question: &str, header: &str) -> Vec<(String, String)> {
    question
        .lines()
        .skip_while(|l| *l != header)
        .skip(1)
        .take_while(|l| l.starts_with("  "))
        .filter_map(item)
        .collect()
}

fn item(line: &str) -> Option<(String, String)> {
    let line = line.strip_prefix("  ")?;
    if line.starts_with(' ') {
        return None;
    }
    let (id, label) = line.split_once(' ').unwrap_or((line, ""));
    Some((id.to_string(), label.trim().to_string()))
}

/// A land first, then a spell, then keep or pass.
fn option(question: &str) -> String {
    let options = last_list(question);
    let first = |f: &dyn Fn(&(String, String)) -> bool| options.iter().find(|o| f(o));
    first(&|(_, label)| label.starts_with("Play land"))
        .or_else(|| first(&|(_, label)| label.starts_with("Cast ")))
        .or_else(|| first(&|(id, _)| id == "keep" || id == "p"))
        .or(options.first())
        .map(|(id, _)| id.clone())
        .unwrap_or_default()
}

/// As few ids as the question allows, from the top of its list.
fn pick(question: &str, shape: &str) -> Vec<String> {
    if shape.contains("or pick=[] for none") {
        return Vec::new();
    }
    let rest = shape.strip_prefix("pick=[").unwrap_or_default();
    let least = rest
        .strip_prefix("exactly ")
        .or_else(|| rest.strip_prefix("from "))
        .and_then(|r| r.split(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or(1);
    last_list(question)
        .into_iter()
        .take(least)
        .map(|(id, _)| id)
        .collect()
}

fn attacks(question: &str) -> Value {
    let at = list_under(question, "Defenders:")
        .into_iter()
        .map(|(id, _)| id)
        .next()
        .unwrap_or_default();
    list_under(question, "Creatures that can attack:")
        .into_iter()
        .map(|(attacker, _)| json!({"attacker": attacker, "at": at}))
        .collect()
}

/// Every pile its least, then the rest in order as far as each takes.
fn piles(question: &str) -> Value {
    let cards: Vec<String> = question
        .lines()
        .find_map(|l| l.strip_prefix("Cards: "))
        .unwrap_or_default()
        .split_whitespace()
        .filter(|w| w.starts_with('#'))
        .map(|w| w.trim_end_matches(',').to_string())
        .collect();
    let bounds: Vec<(usize, usize)> = list_under(question, "Piles:")
        .iter()
        .map(|(_, label)| {
            let within = label
                .split_once('(')
                .and_then(|(_, r)| r.split_once(" card"))
                .map_or("", |(n, _)| n);
            let numbers: Vec<usize> = within.split(' ').filter_map(|w| w.parse().ok()).collect();
            match (within.starts_with("up to"), numbers.as_slice()) {
                (true, [max]) => (0, *max),
                (false, [n]) => (*n, *n),
                (_, [min, max]) => (*min, *max),
                _ => (0, usize::MAX),
            }
        })
        .collect();
    let mut left = cards.into_iter();
    let mut piles: Vec<Vec<String>> = bounds
        .iter()
        .map(|&(min, _)| left.by_ref().take(min).collect())
        .collect();
    for (pile, &(_, max)) in piles.iter_mut().zip(&bounds) {
        let room = max.saturating_sub(pile.len());
        pile.extend(left.by_ref().take(room));
    }
    json!(piles)
}

fn a_type(question: &str) -> String {
    question
        .lines()
        .find_map(|l| {
            l.strip_prefix("QUESTION: Choose a type: ")
                .or_else(|| l.split_once("Types among your cards: ").map(|(_, r)| r))
        })
        .and_then(|r| r.split([',', '.']).next())
        .map_or_else(|| "Human".into(), str::to_string)
}

async fn guest(lobby: &Lobby, name: &str) -> Session {
    lobby
        .guest(&GuestSignIn {
            display_name: name.into(),
            invite_key: None,
        })
        .await
        .unwrap_or_else(|e| panic!("{name} signs in: {e:#}"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn a_language_model_seat_plays_a_game_through_real_sockets() {
    let gateway = spawn_gateway("seat_llm");
    let _agent = attach_agent(&gateway).await;
    let lobby = Lobby::new(&format!("http://127.0.0.1:{}", gateway.port));
    let (base, books) = stand_in().await;

    // The mind as `join --mind anthropic` makes it, from an environment
    // that points it at the stand-in.
    let spec = Spec::parse("anthropic").expect("a spec").expect("a spec");
    let mut settings = Settings::new(&spec);
    // The budget has its own test; here the game is.
    settings.spend_usd = 1000.0;
    settings.spend_tokens = u64::MAX;
    let transcripts = std::env::temp_dir().join(format!(
        "baylee-e2e-seat-llm-{}-{}",
        std::process::id(),
        gateway.port
    ));
    settings.transcripts = Some(transcripts.clone());
    let env = |name: &str| match name {
        "ANTHROPIC_API_KEY" => Some(KEY.to_string()),
        "ANTHROPIC_BASE_URL" => Some(base.clone()),
        _ => None,
    };
    let api = ApiMind::new(
        settings,
        credentials(Provider::Anthropic, &env).expect("a key"),
    );
    let tally = api.tally();
    let llm: Arc<dyn Mind> = Arc::new(api);
    let house: Arc<dyn Mind> = Arc::new(HouseMind::default());
    let llm_name = seat_name(llm.disclosure(), &spec.tag()).unwrap();
    let house_name = seat_name(house.disclosure(), "house").unwrap();
    assert_eq!(llm_name, "LLM-sonnet-5");

    let llm_session = guest(&lobby, &llm_name).await;
    let house_session = guest(&lobby, &house_name).await;
    let llm_deck = Deck::acceptance("Allytifact").unwrap();
    let house_deck = Deck::acceptance("Victory").unwrap();
    let llm_deck_id = lobby.upload(&llm_session, &llm_deck).await.unwrap();
    let house_deck_id = lobby.upload(&house_session, &house_deck).await.unwrap();
    let llm_chair = lobby
        .open(&llm_session, &llm_deck_id, 2, "seat llm")
        .await
        .unwrap();
    let game_id = llm_chair.game_id.clone();
    let house_chair = lobby
        .join(&house_session, &game_id, &house_deck_id, None)
        .await
        .unwrap();
    lobby.ready(&llm_session, &game_id).await.unwrap();
    lobby.ready(&house_session, &game_id).await.unwrap();
    lobby.start(&llm_session, &game_id).await.unwrap();
    lobby
        .wait_for_start(&house_session, &game_id, Duration::from_millis(100))
        .await
        .unwrap();

    let secrets = [
        KEY.to_string(),
        llm_chair.seat_token.clone(),
        house_chair.seat_token.clone(),
    ];
    let options = PlayOptions {
        min_think: Duration::ZERO,
        ..PlayOptions::default()
    };
    let core = |deck: &Deck, mind: &Arc<dyn Mind>| {
        SeatCore::new(
            BridgeConfig::default(),
            deck.list.clone(),
            mind.disclosure(),
        )
    };
    let mut llm_link = SeatLink::new(lobby.clone(), llm_chair, Some(llm_session));
    let mut house_link = SeatLink::new(lobby.clone(), house_chair, Some(house_session));
    let mut llm_transcript = Transcript::memory();
    let mut house_transcript = Transcript::memory();
    let started = std::time::Instant::now();
    let played = tokio::time::timeout(GAME_BUDGET, async {
        tokio::join!(
            bridge::play(
                &mut llm_link,
                core(&llm_deck, &llm),
                llm,
                &mut llm_transcript,
                &options,
            ),
            bridge::play(
                &mut house_link,
                core(&house_deck, &house),
                house,
                &mut house_transcript,
                &options,
            ),
        )
    })
    .await;
    let Ok((llm_played, house_played)) = played else {
        // Where each seat was when the time ran out.
        let tail = |t: &Transcript| {
            let lines = t.lines();
            lines[lines.len().saturating_sub(12)..].join("\n")
        };
        let faults: Vec<String> = lock(&books).faults.iter().take(5).cloned().collect();
        panic!(
            "the game did not end within {GAME_BUDGET:?}; faults {faults:#?}\nthe model's seat:\n{}\nthe house seat:\n{}",
            tail(&llm_transcript),
            tail(&house_transcript)
        );
    };
    let llm_played = llm_played.unwrap_or_else(|e| panic!("the model's seat: {e:#}"));
    let house_played = house_played.unwrap_or_else(|e| panic!("the house seat: {e:#}"));
    let tally = tally.lock().unwrap_or_else(PoisonError::into_inner).clone();
    let books = lock(&books);
    let stats = &llm_played.stats;
    println!(
        "{:.1} s, {} turns; model seat {:?}; {} calls, {:?}; answers {:?}",
        started.elapsed().as_secs_f64(),
        stats.turns,
        stats.outcome,
        tally.calls,
        tally.usage,
        books.shapes
    );
    println!("model seat: {stats:?}");

    // Every request was one the API would take.
    assert!(books.faults.is_empty(), "{:#?}", books.faults);

    // One game, one result, seen the same from both chairs.
    assert!(llm_played.result.is_some(), "{llm_played:?}");
    assert_eq!(llm_played.result, house_played.result);
    let outcomes = (stats.outcome, house_played.stats.outcome);
    assert!(
        matches!(
            outcomes,
            (Some(Outcome::Won), Some(Outcome::Lost)) | (Some(Outcome::Lost), Some(Outcome::Won))
        ),
        "{outcomes:?}"
    );
    assert!(stats.turns > 1, "{stats:?}");
    assert_eq!(llm_played.dials, 1, "the socket never dropped");

    // The model answered, and nothing it sent was refused at the table.
    assert!(stats.asked > 0 && stats.answered.mind > 0, "{stats:?}");
    assert_eq!(stats.refused_by_table, 0, "{stats:?}");
    assert_eq!(stats.unanswerable, 0, "{stats:?}");
    // The provider always answered in time: no question went to the house
    // for want of an answer, and the mind was never taken off the table.
    let fallbacks = &stats.fallbacks;
    assert_eq!(
        (fallbacks.expired, fallbacks.unavailable, fallbacks.down),
        (0, 0, 0),
        "{stats:?}"
    );
    assert_eq!(stats.mind_down, 0, "{stats:?}");
    assert_eq!(tally.failed, 0, "{tally:?}");
    assert!(tally.calls >= SPOILED_CALL, "{tally:?}");
    assert!(tally.usd.is_some_and(|usd| usd > 0.0), "{tally:?}");
    // The id on no list went back as an error, and the game went on.
    assert!(
        books.spoiled.is_some() && books.sent_back,
        "the error was never sent back"
    );

    // What either seat and the mind wrote down names no secret.
    let mind_file = transcripts.join(format!("{game_id}-seat0-mind.jsonl"));
    let mind_lines = std::fs::read_to_string(&mind_file)
        .unwrap_or_else(|e| panic!("{}: {e}", mind_file.display()));
    assert!(
        mind_lines.lines().count() as u64 >= tally.calls,
        "one line per call"
    );
    let seat_lines = llm_transcript
        .lines()
        .iter()
        .chain(house_transcript.lines());
    for line in seat_lines.map(String::as_str).chain(mind_lines.lines()) {
        for secret in &secrets {
            assert!(
                !line.contains(secret.as_str()),
                "a secret was written: {line}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&transcripts);
}
