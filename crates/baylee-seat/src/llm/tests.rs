//! The API mind against a stand-in provider on this machine: no network,
//! no real key. Every reply is scripted, and every request the mind sent
//! is kept for the test to read.

use super::*;
use crate::mind::{Refusal, RefusedBy};
use crate::narrator::tests::{THEM, board, card, context, id, priority, request};
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use baylee_view::LogTail;

/// A key shaped like a real one, so the scrubber's patterns are tested
/// too. Not a key anywhere.
const KEY: &str = "sk-ant-TEST-0123456789abcdefghijklmn";

/// One scripted reply.
#[derive(Clone)]
struct Scripted {
    status: u16,
    body: Value,
    delay: Duration,
}

impl Scripted {
    fn ok(body: Value) -> Self {
        Self {
            status: 200,
            body,
            delay: Duration::ZERO,
        }
    }

    fn slow(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

/// One request the mind sent.
#[derive(Clone, Debug)]
struct Seen {
    path: String,
    key: Option<String>,
    bearer: Option<String>,
    body: Value,
}

#[derive(Clone, Default)]
struct Provider0 {
    replies: Arc<Mutex<VecDeque<Scripted>>>,
    seen: Arc<Mutex<Vec<Seen>>>,
}

impl Provider0 {
    fn seen(&self) -> Vec<Seen> {
        lock(&self.seen).clone()
    }

    fn script(&self, reply: Scripted) {
        lock(&self.replies).push_back(reply);
    }
}

/// Answers the next scripted reply; `ECHO_KEY` in a body is replaced by
/// the key the request carried, as a careless proxy would.
async fn answer(
    State(provider): State<Provider0>,
    uri: Uri,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    };
    let key = header("x-api-key");
    let bearer = header("authorization");
    lock(&provider.seen).push(Seen {
        path: uri.path().to_string(),
        key: key.clone(),
        bearer: bearer.clone(),
        body: serde_json::from_slice(&body).unwrap_or(Value::Null),
    });
    let next = lock(&provider.replies).pop_front();
    let Some(next) = next else {
        let body = json!({"error": {"type": "api_error", "message": "no scripted reply"}});
        return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(body)).into_response();
    };
    tokio::time::sleep(next.delay).await;
    let echoed = key.or(bearer).unwrap_or_default();
    let text = next.body.to_string().replace("ECHO_KEY", &echoed);
    let status = StatusCode::from_u16(next.status).expect("a status");
    (status, [("content-type", "application/json")], text).into_response()
}

/// A stand-in provider on a free port of this machine.
async fn stand_in() -> (String, Provider0) {
    let provider = Provider0::default();
    let app = Router::new().fallback(answer).with_state(provider.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a port");
    let addr = listener.local_addr().expect("an address");
    tokio::spawn(async move { axum::serve(listener, app).await });
    (format!("http://{addr}"), provider)
}

fn env_for(base: &str) -> impl Fn(&str) -> Option<String> + '_ {
    move |name: &str| match name {
        "ANTHROPIC_API_KEY" | "BAYLEE_LLM_API_KEY" => Some(KEY.to_string()),
        "ANTHROPIC_BASE_URL" | "BAYLEE_LLM_BASE_URL" => Some(base.to_string()),
        _ => None,
    }
}

fn mind(base: &str, provider: Provider, tweak: impl FnOnce(&mut Settings)) -> ApiMind {
    let model = match provider {
        Provider::Anthropic => "claude-sonnet-5",
        Provider::OpenAi => "TEST-model",
    };
    let mut settings = Settings::new(&Spec {
        provider,
        model: model.into(),
    });
    tweak(&mut settings);
    let credentials = credentials(provider, &env_for(base)).expect("credentials");
    ApiMind::new(settings, credentials)
}

/// An Anthropic reply: a summarised thought, then the calls.
fn claude(calls: &[(&str, &str, Value)]) -> Value {
    let mut content = vec![json!({
        "type": "thinking", "thinking": "The land first, then the bolt.", "signature": "TEST-sig"
    })];
    for (id, name, input) in calls {
        content.push(json!({"type": "tool_use", "id": id, "name": name, "input": input}));
    }
    json!({
        "id": "msg_TEST", "type": "message", "role": "assistant", "model": "claude-sonnet-5",
        "content": content,
        "stop_reason": if calls.is_empty() { "end_turn" } else { "tool_use" },
        "usage": {"input_tokens": 1200, "output_tokens": 300,
                  "cache_creation_input_tokens": 2000, "cache_read_input_tokens": 0}
    })
}

/// An Anthropic reply with text and no call.
fn claude_text(text: &str) -> Value {
    json!({
        "id": "msg_TEST", "type": "message", "role": "assistant", "model": "claude-sonnet-5",
        "content": [{"type": "text", "text": text}],
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 1000, "output_tokens": 50}
    })
}

fn a_priority() -> Request {
    let (view, log) = board();
    let pending = priority(&view);
    request(view, pending, log)
}

fn empty_log() -> LogTail {
    LogTail {
        from: 9,
        entries: Vec::new(),
    }
}

/// The last user message of a request, as Anthropic content blocks.
fn last_user(seen: &Seen) -> Vec<Value> {
    seen.body["messages"]
        .as_array()
        .and_then(|m| m.last())
        .and_then(|m| m["content"].as_array())
        .cloned()
        .unwrap_or_default()
}

#[tokio::test]
async fn a_valid_answer_is_played_and_the_request_is_the_documented_shape() {
    let (base, provider) = stand_in().await;
    provider.script(Scripted::ok(claude(&[(
        "toolu_1",
        "decide",
        json!({"ask": "q12", "pick": ["a1"], "say": "Land first."}),
    )])));
    let mind = mind(&base, Provider::Anthropic, |_| {});
    let answer = mind.decide(a_priority()).await.expect("an answer");
    assert_eq!(answer.action, PlayerAction::PlayLand { card: id(52) });
    let note: Value = serde_json::from_str(answer.note.as_deref().expect("a note")).expect("json");
    assert_eq!(note["say"], "Land first.");
    assert_eq!(note["reasoning"], "The land first, then the bolt.");
    assert!(
        note["chose"]
            .as_str()
            .expect("chose")
            .starts_with("a1 Play land")
    );
    let seen = provider.seen();
    assert_eq!(seen.len(), 1);
    let sent = &seen[0];
    assert_eq!(sent.path, "/v1/messages");
    assert_eq!(sent.key.as_deref(), Some(KEY));
    let body = &sent.body;
    assert_eq!(body["model"], "claude-sonnet-5");
    assert_eq!(body["thinking"]["type"], "adaptive");
    assert_eq!(body["output_config"]["effort"], "medium");
    assert_eq!(body["tool_choice"]["type"], "auto");
    assert_eq!(body["system"][0]["cache_control"]["type"], "ephemeral");
    let blocks = last_user(sent);
    assert_eq!(blocks.len(), 2, "the prefix and the decision");
    assert!(
        blocks[0]["text"]
            .as_str()
            .expect("prefix")
            .starts_with("THE GAME")
    );
    assert_eq!(blocks[0]["cache_control"]["type"], "ephemeral");
    assert!(
        blocks[1]["text"]
            .as_str()
            .expect("wake")
            .starts_with("DECISION q12")
    );
    let tally = lock(&mind.tally()).clone();
    assert_eq!(tally.calls, 1);
    assert_eq!(tally.usage.total(), 3500);
    assert!(tally.usd.expect("a known price") > 0.0);
}

/// A cast that taps lands is one answer: the taps, the cast and the
/// target named ahead go out without another call.
#[tokio::test]
async fn a_cast_by_tapping_plays_out_without_another_call() {
    let (base, provider) = stand_in().await;
    provider.script(Scripted::ok(claude(&[(
        "toolu_1",
        "decide",
        json!({"ask": "q12", "pick": "a2", "then": {"targets": ["#45"]}}),
    )])));
    let mind = mind(&base, Provider::Anthropic, |_| {});
    let first = mind.decide(a_priority()).await.expect("an answer");
    assert_eq!(
        first.action,
        PlayerAction::ActivateManaAbility { source: id(23) }
    );
    // The Mountain's mana floats; the table asks again, and the bolt is
    // castable from the pool.
    let (mut view, _) = board();
    view.seats[0].mana_pool.red = 1;
    let mut pending = priority(&view);
    if let Pending::Priority { legal, .. } = &mut pending {
        legal.castable = vec![id(50)];
        legal.mana_abilities.retain(|m| *m != id(23));
    }
    let mut again = request(view, pending, empty_log());
    again.question = 13;
    again.continuing = true;
    let cast = mind.decide(again).await.expect("the cast");
    assert_eq!(cast.action, PlayerAction::CastSpell { card: id(50) });
    // The bolt asks for its target.
    let (mut view, _) = board();
    let mut bolt = card(50, "Lightning Bolt");
    bolt.stack_item = Some(baylee_view::StackItem::Spell);
    view.stack = vec![bolt.clone()];
    view.targeting = Some(baylee_view::TargetingContext {
        source: bolt,
        text: None,
        whole_spell: true,
        second: false,
        batch_count: 1,
    });
    let pending = Pending::ChooseTargets {
        player: context().seat,
        options: vec![id(30), id(31), id(45), id(46)],
        player_options: vec![context().seat, THEM],
        min: 1,
        max: 1,
        reason: TargetPrompt::Targets,
    };
    let mut targets = request(view, pending, empty_log());
    targets.question = 14;
    let target = mind.decide(targets).await.expect("the target");
    assert_eq!(
        target.action,
        PlayerAction::ChooseTargets {
            objects: vec![id(45)],
            players: Vec::new()
        }
    );
    assert_eq!(provider.seen().len(), 1, "one call for all three");
}

/// An answer the menu cannot read goes back once, with the reason; a
/// second unreadable answer is the house's to replace.
#[tokio::test]
async fn a_malformed_answer_is_sent_back_once() {
    let (base, provider) = stand_in().await;
    provider.script(Scripted::ok(claude_text("I would play the Mountain.")));
    provider.script(Scripted::ok(claude(&[(
        "toolu_2",
        "decide",
        json!({"ask": "q12", "pick": ["a1"]}),
    )])));
    let mind = mind(&base, Provider::Anthropic, |_| {});
    let answer = mind.decide(a_priority()).await.expect("the second try");
    assert_eq!(answer.action, PlayerAction::PlayLand { card: id(52) });
    let seen = provider.seen();
    assert_eq!(seen.len(), 2);
    let messages = seen[1].body["messages"].as_array().expect("messages");
    assert_eq!(messages.len(), 3, "the decision, the reply, the reason");
    assert_eq!(messages[1]["role"], "assistant");
    let reason = messages[2]["content"][0]["text"].as_str().expect("text");
    assert!(reason.contains("decide tool"), "{reason}");

    // Twice unreadable: declined, and the house answers.
    provider.script(Scripted::ok(claude(&[(
        "toolu_3",
        "decide",
        json!({"ask": "q12", "pick": ["a99"]}),
    )])));
    provider.script(Scripted::ok(claude(&[(
        "toolu_4",
        "decide",
        json!({"ask": "q12", "pick": ["a98"]}),
    )])));
    let mut next = a_priority();
    next.log = empty_log();
    let refused = mind.decide(next).await.expect_err("unreadable twice");
    assert!(
        matches!(&refused, MindError::Declined(why) if why.contains("a99") || why.contains("a98")),
        "{refused:?}"
    );
    // The second try answered the first's error result.
    let seen = provider.seen();
    let blocks = last_user(&seen[3]);
    assert_eq!(blocks[0]["type"], "tool_result");
    assert_eq!(blocks[0]["tool_use_id"], "toolu_3");
    assert_eq!(blocks[0]["is_error"], true);
}

/// The table refuses an answer: the model is asked again with the reason,
/// in the tool result of the call it made.
#[tokio::test]
async fn a_refused_answer_is_asked_again_with_the_reason() {
    let (base, provider) = stand_in().await;
    provider.script(Scripted::ok(claude(&[(
        "toolu_1",
        "decide",
        json!({"ask": "q12", "pick": ["a1"]}),
    )])));
    provider.script(Scripted::ok(claude(&[(
        "toolu_2",
        "decide",
        json!({"ask": "q12", "pick": ["p"]}),
    )])));
    let mind = mind(&base, Provider::Anthropic, |_| {});
    let first = mind.decide(a_priority()).await.expect("an answer");
    let mut retry = a_priority();
    retry.log = empty_log();
    retry.retry = Some(Refusal {
        answer: first.action,
        reason: "TEST: the land drop is used".into(),
        by: RefusedBy::Table,
    });
    let second = mind.decide(retry).await.expect("the second answer");
    assert_eq!(second.action, PlayerAction::PassPriority);
    let seen = provider.seen();
    let blocks = last_user(&seen[1]);
    assert_eq!(blocks[0]["type"], "tool_result");
    assert_eq!(blocks[0]["tool_use_id"], "toolu_1");
    assert_eq!(blocks[0]["is_error"], true);
    assert!(
        blocks[0]["content"]
            .as_str()
            .expect("content")
            .contains("the land drop is used")
    );
    // Append-only: the first exchange is replayed as it was.
    let messages = seen[1].body["messages"].as_array().expect("messages");
    assert_eq!(messages[0], seen[0].body["messages"][0]);
    assert_eq!(messages[1]["content"][0]["signature"], "TEST-sig");
}

/// A provider slower than the question's time: the call gives up before
/// the bridge's deadline and the house answers; an answer that arrives
/// after the bridge stopped waiting is said in the next message.
#[tokio::test]
async fn a_slow_provider_leaves_the_question_to_the_house() {
    let (base, provider) = stand_in().await;
    provider.script(
        Scripted::ok(claude(&[(
            "toolu_1",
            "decide",
            json!({"ask": "q12", "pick": ["a1"]}),
        )]))
        .slow(Duration::from_secs(3)),
    );
    let mind = mind(&base, Provider::Anthropic, |_| {});
    let mut hurried = a_priority();
    hurried.budget = Duration::from_secs(2);
    let started = Instant::now();
    let error = mind.decide(hurried).await.expect_err("too slow");
    assert!(matches!(error, MindError::Unavailable(_)), "{error:?}");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "gave up in time"
    );

    // The bridge drops the future; the reply still comes, and is counted.
    provider.script(
        Scripted::ok(claude(&[(
            "toolu_2",
            "decide",
            json!({"ask": "q13", "pick": ["a1"]}),
        )]))
        .slow(Duration::from_millis(600)),
    );
    let mut dropped = a_priority();
    dropped.question = 13;
    dropped.log = empty_log();
    let gone = tokio::time::timeout(Duration::from_millis(100), mind.decide(dropped)).await;
    assert!(gone.is_err(), "the bridge stopped waiting");
    tokio::time::sleep(Duration::from_millis(1200)).await;
    provider.script(Scripted::ok(claude(&[(
        "toolu_3",
        "decide",
        json!({"ask": "q14", "pick": ["p"]}),
    )])));
    let mut next = a_priority();
    next.question = 14;
    next.log = empty_log();
    mind.decide(next).await.expect("an answer");
    let seen = provider.seen();
    let text = last_user(seen.last().expect("a request"))
        .iter()
        .filter_map(|b| b["text"].as_str().map(str::to_string))
        .collect::<String>();
    assert!(text.contains("q13 came after its time ran out"), "{text}");
    assert_eq!(lock(&mind.tally()).calls, 3);
}

/// No key, no mind: the credentials refuse before any request, and a key
/// is never sent in the clear to another machine.
#[test]
fn no_key_no_mind() {
    let none = |_: &str| None;
    let refused = credentials(Provider::Anthropic, &none).expect_err("no key");
    assert!(refused.contains("ANTHROPIC_API_KEY"), "{refused}");
    let remote_openai = |name: &str| {
        (name == "BAYLEE_LLM_BASE_URL").then(|| "https://api.example.com/v1".to_string())
    };
    assert!(credentials(Provider::OpenAi, &remote_openai).is_err());
    let local = |name: &str| {
        (name == "BAYLEE_LLM_BASE_URL").then(|| "http://127.0.0.1:8080/v1".to_string())
    };
    assert!(
        credentials(Provider::OpenAi, &local).is_ok(),
        "a local server needs no key"
    );
    let clear = |name: &str| match name {
        "ANTHROPIC_API_KEY" => Some(KEY.to_string()),
        "ANTHROPIC_BASE_URL" => Some("http://api.example.com".to_string()),
        _ => None,
    };
    assert!(
        credentials(Provider::Anthropic, &clear).is_err(),
        "http:// to another machine"
    );
    let fine = credentials(Provider::Anthropic, &|name: &str| {
        (name == "ANTHROPIC_API_KEY").then(|| KEY.to_string())
    })
    .expect("a key");
    assert!(!format!("{fine:?}").contains("0123456789"));
}

/// A provider that echoes the key back in its error: the key reaches no
/// error, no transcript line and no note.
#[tokio::test]
async fn no_key_reaches_an_error_or_a_transcript() {
    let (base, provider) = stand_in().await;
    provider.script(Scripted {
        status: 401,
        body: json!({"type": "error", "error": {
            "type": "authentication_error", "message": "invalid x-api-key: ECHO_KEY"
        }}),
        delay: Duration::ZERO,
    });
    let dir = std::env::temp_dir().join(format!("baylee-seat-llm-{}", std::process::id()));
    let mind = mind(&base, Provider::Anthropic, |s| {
        s.transcripts = Some(dir.clone());
    });
    let error = mind.decide(a_priority()).await.expect_err("refused key");
    let said = error.to_string();
    assert!(said.contains("401"), "{said}");
    assert!(said.contains("authentication_error"), "{said}");
    assert!(!said.contains("0123456789"), "{said}");
    // The stand-in did echo it: the scrubber is what kept it out.
    assert_eq!(provider.seen()[0].key.as_deref(), Some(KEY));
    let transcript =
        std::fs::read_to_string(dir.join("golden-seat0-mind.jsonl")).expect("a transcript");
    assert!(transcript.contains("401"), "{transcript}");
    assert!(!transcript.contains("0123456789"), "{transcript}");
    assert!(!format!("{:?}", mind.credentials).contains("0123456789"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A redirect is followed nowhere. ureq drops only `Authorization` and
/// `Cookie` when it follows one, so an `x-api-key` would have gone to the
/// address it named: a call and a probe that meet one fail, by the status
/// alone, and nothing reaches the other listener.
#[tokio::test]
async fn a_redirect_is_not_followed_and_the_key_goes_nowhere_else() {
    let moved = json!({"type": "error", "error": {"type": "moved", "message": "see LOCATION"}});
    // A POST follows 301–303 as a GET; a GET (the probe) follows all five.
    for status in [301, 302, 303, 307, 308] {
        for provider in [Provider::Anthropic, Provider::OpenAi] {
            let redirect = crate::testnet::redirect(status, &moved).await;
            let mind = mind(&redirect.base, provider, |_| {});
            let error = mind.decide(a_priority()).await.expect_err("a redirect");
            let said = error.to_string();
            assert!(said.contains(&status.to_string()), "{said}");
            assert!(said.contains("redirect"), "{said}");
            assert!(!said.contains("0123456789"), "the key: {said}");
            assert!(
                !said.contains(&redirect.target_port),
                "the location: {said}"
            );
            assert!(!said.contains("see "), "the body: {said}");
            assert!(!mind.probe().await, "a redirected probe is no answer");
            // The key did go out, to the address it was given: the test
            // would see it had it gone further.
            let asked = redirect.asked();
            assert_eq!(asked.len(), 2, "the call and the probe");
            let name = match provider {
                Provider::Anthropic => "x-api-key",
                Provider::OpenAi => "authorization",
            };
            assert!(asked.iter().all(|headers| headers.contains_key(name)));
            assert_eq!(redirect.followed(), 0, "{status} {provider:?} was followed");
            assert_eq!(lock(&mind.tally()).failed, 1);
        }
    }
}

/// Every agent the mind builds follows no redirect, and talks to another
/// machine only over TLS.
#[test]
fn the_agent_follows_no_redirect_and_leaves_this_machine_only_over_tls() {
    let at = |base: &'static str| {
        let credentials = credentials(Provider::Anthropic, &move |name: &str| match name {
            "ANTHROPIC_API_KEY" => Some(KEY.to_string()),
            "ANTHROPIC_BASE_URL" => Some(base.to_string()),
            _ => None,
        })
        .expect("credentials");
        let settings = Settings::new(&Spec {
            provider: Provider::Anthropic,
            model: "claude-sonnet-5".into(),
        });
        ApiMind::new(settings, credentials)
    };
    let remote = at("https://api.example.com");
    assert_eq!(remote.agent.config().max_redirects(), 0);
    assert!(remote.agent.config().https_only());
    let local = at("http://127.0.0.1:8080");
    assert_eq!(local.agent.config().max_redirects(), 0);
    assert!(
        !local.agent.config().https_only(),
        "this machine, over http"
    );
}

/// Past the game's budget, every question is the house's, without a call.
#[tokio::test]
async fn past_the_budget_the_house_finishes() {
    let (base, provider) = stand_in().await;
    provider.script(Scripted::ok(claude(&[(
        "toolu_1",
        "decide",
        json!({"ask": "q12", "pick": ["a1"]}),
    )])));
    let mind = mind(&base, Provider::Anthropic, |s| s.spend_tokens = 1_000);
    mind.decide(a_priority()).await.expect("the first answer");
    let mut next = a_priority();
    next.log = empty_log();
    let spent = mind.decide(next).await.expect_err("spent");
    assert!(
        matches!(&spent, MindError::Declined(why) if why.contains("budget")),
        "{spent:?}"
    );
    assert_eq!(provider.seen().len(), 1);
    assert!(lock(&mind.tally()).spent);
}

/// A dollar budget is held only where the price is known: this build's, or
/// one given. A model with neither plays only under a token budget stated
/// as its limit.
#[test]
fn a_dollar_budget_is_held_only_with_a_price() {
    let spec = |provider, model: &str| Spec {
        provider,
        model: model.into(),
    };
    let known = Settings::new(&spec(Provider::Anthropic, "claude-sonnet-5"));
    assert_eq!(known.price, price("claude-sonnet-5"));
    assert_eq!(known.spend_usd, Some(DEFAULT_SPEND_USD));
    let mut defaults = known.clone();
    defaults.budget(None, None, None).expect("a priced model");
    assert_eq!(
        (defaults.spend_usd, defaults.spend_tokens),
        (Some(DEFAULT_SPEND_USD), DEFAULT_SPEND_TOKENS)
    );

    let unknown = Settings::new(&spec(Provider::OpenAi, "TEST-model"));
    assert_eq!((unknown.price, unknown.spend_usd), (None, None));
    let refused = unknown.clone().budget(None, None, None).unwrap_err();
    for named in ["TEST-model", "--price-in", "--price-out", "--spend-tokens"] {
        assert!(refused.contains(named), "{refused}");
    }
    for tokens in [None, Some(1_000)] {
        let refused = unknown.clone().budget(None, Some(3.0), tokens).unwrap_err();
        assert!(refused.contains("--spend-usd cannot be held"), "{refused}");
    }
    let mut tokens_only = unknown.clone();
    tokens_only
        .budget(None, None, Some(1_000))
        .expect("a token limit");
    assert_eq!(
        (
            tokens_only.price,
            tokens_only.spend_usd,
            tokens_only.spend_tokens
        ),
        (None, None, 1_000)
    );
    let mut priced = unknown;
    let given = Price::per_million(0.3, 1.2);
    priced.budget(Some(given), None, None).expect("a price");
    assert_eq!(
        (priced.price, priced.spend_usd, priced.spend_tokens),
        (Some(given), Some(DEFAULT_SPEND_USD), DEFAULT_SPEND_TOKENS)
    );

    // A given price errs high on the cache: a write at this build's ratio,
    // a read at no discount.
    for model in ["claude-sonnet-5-5", "claude-sonnet-5", "claude-opus-5-5"] {
        let table = price(model).expect("a price");
        let given = Price::per_million(table.input, table.output);
        assert!(
            (given.cache_write - table.cache_write).abs() < 1e-9,
            "{model}"
        );
        assert!(given.cache_read >= table.cache_read, "{model}");
    }
}

/// A price given for a model is the one its dollars are counted at, and
/// past its dollar budget the house finishes.
#[tokio::test]
async fn a_given_price_is_counted_and_its_budget_held() {
    let (base, provider) = stand_in().await;
    provider.script(Scripted::ok(claude(&[(
        "toolu_1",
        "decide",
        json!({"ask": "q12", "pick": ["a1"]}),
    )])));
    // 3,500 tokens at $1,000 a million each way: some dollars, over one.
    let mind = mind(&base, Provider::Anthropic, |s| {
        s.budget(Some(Price::per_million(1000.0, 1000.0)), Some(1.0), None)
            .expect("a price");
    });
    mind.decide(a_priority()).await.expect("the first answer");
    let usd = lock(&mind.tally()).usd.expect("a price");
    assert!(usd > 1.0, "{usd}");
    let mut next = a_priority();
    next.log = empty_log();
    let spent = mind.decide(next).await.expect_err("spent");
    assert!(
        matches!(&spent, MindError::Declined(why) if why.contains("$1.00")),
        "{spent:?}"
    );
    assert_eq!(provider.seen().len(), 1);
}

/// A new turn starts a new conversation, carrying the model's own notes.
#[tokio::test]
async fn a_new_turn_starts_a_new_conversation_with_the_notes() {
    let (base, provider) = stand_in().await;
    for (id, pick) in [("toolu_1", "a1"), ("toolu_2", "p")] {
        provider.script(Scripted::ok(claude(&[(
            id,
            "decide",
            json!({"ask": "q12", "pick": [pick], "say": format!("TEST note {pick}")}),
        )])));
    }
    let mind = mind(&base, Provider::Anthropic, |_| {});
    mind.decide(a_priority()).await.expect("turn 7");
    let (mut view, _) = board();
    view.turn = 8;
    let pending = priority(&view);
    mind.decide(request(view, pending, empty_log()))
        .await
        .expect("turn 8");
    let seen = provider.seen();
    let messages = seen[1].body["messages"].as_array().expect("messages");
    assert_eq!(messages.len(), 1, "a fresh conversation");
    let blocks = last_user(&seen[1]);
    assert!(
        blocks[0]["text"]
            .as_str()
            .expect("prefix")
            .starts_with("THE GAME")
    );
    let wake = blocks[1]["text"].as_str().expect("wake");
    assert!(
        wake.contains("Your notes from earlier turns:\n  - TEST note a1"),
        "{wake}"
    );
}

/// A conversation the provider turned down is not sent again: the next
/// question, in the same turn, starts a new one.
#[tokio::test]
async fn a_conversation_the_provider_turned_down_is_started_again() {
    let (base, provider) = stand_in().await;
    provider.script(Scripted::ok(claude(&[(
        "toolu_1",
        "decide",
        json!({"ask": "q12", "pick": ["a1"], "say": "TEST note a1"}),
    )])));
    provider.script(Scripted {
        status: 400,
        body: json!({"type": "error", "error": {
            "type": "invalid_request_error", "message": "TEST: messages.1: a bad replay"
        }}),
        delay: Duration::ZERO,
    });
    provider.script(Scripted::ok(claude(&[(
        "toolu_3",
        "decide",
        json!({"ask": "q12", "pick": ["p"]}),
    )])));
    let mind = mind(&base, Provider::Anthropic, |_| {});
    mind.decide(a_priority()).await.expect("the first answer");
    let refused = mind.decide(a_priority()).await;
    assert!(
        matches!(&refused, Err(MindError::Unavailable(why)) if why.starts_with("400:")),
        "{refused:?}"
    );
    mind.decide(a_priority()).await.expect("the third answer");
    let seen = provider.seen();
    let lengths: Vec<usize> = seen
        .iter()
        .map(|s| s.body["messages"].as_array().map_or(0, Vec::len))
        .collect();
    assert_eq!(lengths, [1, 3, 1], "the third call starts again");
    let blocks = last_user(&seen[2]);
    assert!(
        blocks[0]["text"]
            .as_str()
            .is_some_and(|t| t.starts_with("THE GAME"))
    );
    let wake = blocks[1]["text"].as_str().expect("wake");
    assert!(wake.contains("  - TEST note a1"), "{wake}");
}

/// A turn whose conversation has grown past its bound starts a new one at
/// its next question, the notes carried over; under the bound it grows.
#[tokio::test]
async fn a_long_turn_starts_a_new_conversation() {
    let (base, provider) = stand_in().await;
    for call in 1..=4 {
        provider.script(Scripted::ok(claude(&[(
            &format!("toolu_{call}"),
            "decide",
            json!({"ask": "q12", "pick": ["a1"], "say": format!("TEST note {call}")}),
        )])));
    }
    let roomy = mind(&base, Provider::Anthropic, |_| {});
    roomy.decide(a_priority()).await.expect("the first answer");
    roomy.decide(a_priority()).await.expect("the second answer");
    // Bounded under one decision and its answer.
    let tight = mind(&base, Provider::Anthropic, |s| s.conversation_tokens = 1);
    tight.decide(a_priority()).await.expect("the first answer");
    tight.decide(a_priority()).await.expect("the second answer");
    let seen = provider.seen();
    let lengths: Vec<usize> = seen
        .iter()
        .map(|s| s.body["messages"].as_array().map_or(0, Vec::len))
        .collect();
    assert_eq!(lengths, [1, 3, 1, 1]);
    let wake = last_user(&seen[3])[1]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(wake.contains("  - TEST note 3"), "{wake}");
}

/// A cast the table took back is said at the next priority: without a
/// word the model sees the same question and may answer it the same way.
/// A cast that happened is not.
#[tokio::test]
async fn a_cast_the_table_took_back_is_said() {
    let (base, provider) = stand_in().await;
    // Lightning Bolt castable with {R} floating: the option is the cast
    // itself, not a plan of taps.
    let bolt = || {
        let (mut view, log) = board();
        view.seats[0].mana_pool.red = 1;
        let mut pending = priority(&view);
        if let Pending::Priority { legal, .. } = &mut pending {
            legal.castable.push(id(50));
        }
        request(view, pending, log)
    };
    let wake = Narrator::new(&bolt().context).wake(&bolt(), &[]);
    let option = wake
        .text
        .lines()
        .find(|l| l.contains("Cast Lightning Bolt #50"))
        .and_then(|l| l.split_whitespace().next())
        .expect("the Bolt is offered")
        .to_string();
    for call in 1..=4 {
        let pick = if call % 2 == 1 { option.as_str() } else { "p" };
        provider.script(Scripted::ok(claude(&[(
            &format!("toolu_{call}"),
            "decide",
            json!({"ask": "q12", "pick": [pick]}),
        )])));
    }
    let mind = mind(&base, Provider::Anthropic, |_| {});
    let cast = mind.decide(bolt()).await.expect("the cast");
    assert_eq!(cast.action, PlayerAction::CastSpell { card: id(50) });
    // The next priority, and the Bolt is still in the hand.
    mind.decide(bolt()).await.expect("a pass");
    let told = |seen: &Seen| {
        last_user(seen)
            .iter()
            .filter_map(|b| b["text"].as_str())
            .any(|t| t.contains("Your cast of Lightning Bolt #50 did not happen"))
    };
    let seen = provider.seen();
    assert!(!told(&seen[0]));
    assert!(told(&seen[1]), "{:?}", last_user(&seen[1]));
    // Cast again, and this time the log says it was cast, though the Bolt
    // is back in the hand (an object keeps its handle across zones, so its
    // place alone would say the cast never happened).
    mind.decide(bolt()).await.expect("the cast again");
    let mut cast_and_back = bolt();
    let from = u32::try_from(cast_and_back.log.entries.len()).expect("a short log");
    cast_and_back.log = LogTail {
        from,
        entries: vec![baylee_view::LogEntry {
            turn: 7,
            repeat: 1,
            at: 0,
            event: LogEvent::Cast {
                player: crate::narrator::tests::ME,
                spell: LogObject::Known {
                    id: id(50),
                    card: None,
                    token: None,
                    name: "Lightning Bolt".into(),
                },
                from: None,
            },
        }],
    };
    mind.decide(cast_and_back).await.expect("a pass");
    assert!(!told(&provider.seen()[3]));
}

/// The OpenAI-compatible path: a function call whose arguments are a JSON
/// string, the bearer key, and the JSON mode for a server without tools.
#[tokio::test]
async fn an_openai_compatible_endpoint_answers_by_function_or_by_json() {
    let (base, provider) = stand_in().await;
    provider.script(Scripted::ok(json!({
        "choices": [{"message": {
            "role": "assistant", "content": null, "reasoning_content": "Land first.",
            "tool_calls": [{"id": "call_1", "type": "function", "function": {
                "name": "decide", "arguments": "{\"ask\":\"q12\",\"pick\":[\"a1\"]}"
            }}]
        }, "finish_reason": "tool_calls"}],
        "usage": {"prompt_tokens": 900, "completion_tokens": 40}
    })));
    let tools = mind(&base, Provider::OpenAi, |_| {});
    let answer = tools.decide(a_priority()).await.expect("an answer");
    assert_eq!(answer.action, PlayerAction::PlayLand { card: id(52) });
    let seen = provider.seen();
    assert_eq!(seen[0].path, "/chat/completions");
    assert_eq!(
        seen[0].bearer.as_deref(),
        Some(format!("Bearer {KEY}").as_str())
    );
    assert_eq!(seen[0].body["messages"][0]["role"], "system");
    assert!(seen[0].body["tools"].is_array());

    provider.script(Scripted::ok(json!({
        "choices": [{"message": {"role": "assistant",
            "content": "```json\n{\"ask\": \"q12\", \"pick\": [\"p\"], \"say\": \"Hold.\"}\n```"},
            "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 900, "completion_tokens": 40}
    })));
    let json_mind = mind(&base, Provider::OpenAi, |s| s.answer = AnswerMode::Json);
    let answer = json_mind.decide(a_priority()).await.expect("a JSON answer");
    assert_eq!(answer.action, PlayerAction::PassPriority);
    let seen = provider.seen();
    assert_eq!(seen[1].body["response_format"]["type"], "json_object");
    assert!(seen[1].body.get("tools").is_none());
}

#[test]
fn a_spec_names_a_provider_and_a_model() {
    let default = Spec::parse("anthropic").expect("anthropic").expect("valid");
    assert_eq!(default.model, DEFAULT_ANTHROPIC_MODEL);
    assert_eq!(default.tag(), "sonnet-5-5");
    let opus = Spec::parse("anthropic:claude-opus-5-5")
        .expect("anthropic")
        .expect("valid");
    assert_eq!(opus.tag(), "opus-5-5");
    let deepseek = Spec::parse("openai:deepseek-chat")
        .expect("openai")
        .expect("valid");
    assert_eq!(deepseek.provider, Provider::OpenAi);
    assert_eq!(deepseek.tag(), "deepseek");
    assert!(Spec::parse("openai").expect("openai").is_err());
    assert!(
        Spec::parse("anthropic:bad model")
            .expect("anthropic")
            .is_err()
    );
    assert!(Spec::parse("house").is_none());
    assert_eq!(
        Spec::parse("openai:org/Qwen3.5-32B")
            .expect("openai")
            .expect("valid")
            .tag(),
        "Qwen3-5-32B"
    );
}

/// Under a hard limit (a reservation in the spend book) a call is held at
/// the most it can cost before it is sent, and one that could pass the
/// game's budget is not sent at all.
#[tokio::test]
async fn under_a_hard_limit_no_call_is_sent_that_could_pass_the_budget() {
    let (base, provider) = stand_in().await;
    // A reply of 16,000 tokens at $10 a million is $0.16 on its own.
    let tight = mind(&base, Provider::Anthropic, |s| {
        s.hard_limit = true;
        s.spend_usd = Some(0.10);
    });
    let refused = tight.decide(a_priority()).await.expect_err("could pass it");
    assert!(
        matches!(&refused, MindError::Declined(why) if why.contains("budget of $0.10 cannot hold")),
        "{refused:?}"
    );
    assert!(provider.seen().is_empty(), "nothing was sent");
    let tally = lock(&tight.tally()).clone();
    assert!(tally.spent && tally.calls == 0 && tally.held == Worst::default());

    // The same budget in tokens.
    let tokens = mind(&base, Provider::Anthropic, |s| {
        s.hard_limit = true;
        s.spend_tokens = 17_000;
    });
    let refused = tokens
        .decide(a_priority())
        .await
        .expect_err("could pass it");
    assert!(
        matches!(&refused, MindError::Declined(why) if why.contains("17000 tokens cannot hold")),
        "{refused:?}"
    );
    assert!(provider.seen().is_empty());

    // With room for it, the call goes, and what was held comes back as the
    // bill.
    provider.script(Scripted::ok(claude(&[(
        "toolu_1",
        "decide",
        json!({"ask": "q12", "pick": ["a1"]}),
    )])));
    let roomy = mind(&base, Provider::Anthropic, |s| {
        s.hard_limit = true;
        s.spend_usd = Some(1.0);
    });
    roomy.decide(a_priority()).await.expect("an answer");
    let tally = lock(&roomy.tally()).clone();
    assert_eq!((tally.calls, tally.held.tokens), (1, 0));
    assert_eq!(tally.spend_tokens(), tally.usage.total());
    let sent = serde_json::to_vec(&provider.seen()[0].body).unwrap().len();
    let worst = roomy.settings().worst(sent);
    assert!(worst.tokens >= tally.usage.total(), "{worst:?} {tally:?}");
    assert!(
        worst.usd.unwrap() >= tally.usd.unwrap(),
        "{worst:?} {tally:?}"
    );
}

/// A call whose bill nobody knows (it timed out after it was sent) counts
/// at its worst in what the game may have spent; one the provider answered
/// with an error counts nothing.
#[tokio::test]
async fn a_call_whose_bill_is_unknown_counts_at_its_worst() {
    let (base, provider) = stand_in().await;
    provider.script(
        Scripted::ok(claude(&[(
            "toolu_1",
            "decide",
            json!({"ask": "q12", "pick": ["a1"]}),
        )]))
        .slow(Duration::from_secs(3)),
    );
    let mind = mind(&base, Provider::Anthropic, |s| s.hard_limit = true);
    let mut hurried = a_priority();
    hurried.budget = Duration::from_secs(2);
    let error = mind.decide(hurried).await.expect_err("too slow");
    assert!(matches!(error, MindError::Unavailable(_)), "{error:?}");
    let tally = lock(&mind.tally()).clone();
    assert_eq!((tally.calls, tally.failed, tally.usage.total()), (1, 1, 0));
    assert!(tally.unsure.tokens > 16_000, "{tally:?}");
    assert_eq!(tally.spend_tokens(), tally.unsure.tokens);
    assert_eq!(tally.spend_usd(), tally.unsure.usd);
    assert!(tally.spend_usd().unwrap() > 0.16, "{tally:?}");

    provider.script(Scripted {
        status: 500,
        body: json!({"error": {"type": "api_error", "message": "overloaded"}}),
        delay: Duration::ZERO,
    });
    let mut next = a_priority();
    next.question = 13;
    next.log = empty_log();
    let error = mind.decide(next).await.expect_err("an error");
    assert!(matches!(error, MindError::Unavailable(_)), "{error:?}");
    let after = lock(&mind.tally()).clone();
    assert_eq!(after.failed, 2);
    assert_eq!(
        after.unsure, tally.unsure,
        "an error answered is not billed"
    );
    assert_eq!(after.held, Worst::default());
}
