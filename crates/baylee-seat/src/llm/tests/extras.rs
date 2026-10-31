//! Reports and lookups (`docs/llm-protocol.md` §"Reports and lookups"):
//! what the prompt and the schema say of them, and how the mind acts on
//! them.

use super::*;
use crate::lookup::scryfall::Scryfall;
use crate::lookup::tests::stand_in_scryfall;
use crate::reports;
use prompt::Extras;

/// The prompt and the schemas as they were before `report` and `lookup`
/// (`tests/prompt_before.json`, written by the build before them).
fn before() -> Value {
    serde_json::from_str(include_str!("prompt_before.json")).expect("the frozen prompt")
}

/// `schema` without the two fields every seat now has.
fn without_extras(mut schema: Value) -> Value {
    let properties = schema["properties"].as_object_mut().expect("properties");
    assert!(properties.remove("report").is_some(), "report is there");
    assert!(properties.remove("lookup").is_some(), "lookup is there");
    schema
}

/// `report` and `lookup` are added to what every seat was told, and
/// nothing else of it changed by a byte: the schemas without them, the
/// tools, the concession and the system prompt's common part are the
/// frozen ones.
#[test]
fn reports_and_lookups_are_added_and_nothing_else_changed() {
    let before = before();
    for extras in [Extras::default(), Extras { scryfall: true }] {
        assert_eq!(
            without_extras(prompt::decide_schema(extras)).to_string(),
            before["decide_schema"].to_string()
        );
        assert_eq!(
            without_extras(prompt::answer_schema(extras)).to_string(),
            before["answer_schema"].to_string()
        );
        let mut tools = prompt::anthropic_tools(extras);
        tools[0]["input_schema"] = without_extras(tools[0]["input_schema"].take());
        assert_eq!(tools.to_string(), before["anthropic_tools"].to_string());
        let mut tools = prompt::openai_tools(extras);
        let parameters = tools[0]["function"]["parameters"].take();
        tools[0]["function"]["parameters"] = without_extras(parameters);
        assert_eq!(tools.to_string(), before["openai_tools"].to_string());
        let system = prompt::system(extras);
        assert!(system.starts_with(before["system"].as_str().unwrap()));
        assert!(
            system.contains("REPORTING A RULES OR ENGINE ERROR"),
            "{system}"
        );
        assert!(system.contains("LOOKING A CARD UP"), "{system}");
        assert_eq!(system.contains("Scryfall"), extras.scryfall, "{system}");
    }
    assert_eq!(
        prompt::concede_schema().to_string(),
        before["concede_schema"].to_string()
    );
    assert_eq!(prompt::JSON_MODE, before["json_mode"].as_str().unwrap());
    assert_eq!(prompt::SYSTEM, before["system"].as_str().unwrap());
}

/// The fields say what they are for; only a profile with `lookup:
/// "scryfall"` is told of rulings.
#[test]
fn the_schema_says_what_a_report_and_a_lookup_are_for() {
    let schema = prompt::answer_schema(Extras::default());
    let report = &schema["properties"]["report"];
    assert_eq!(report["required"], json!(["what"]));
    for field in ["what", "expected", "evidence", "references"] {
        assert!(report["properties"].get(field).is_some(), "{field}");
    }
    let said = report["description"].as_str().unwrap();
    assert!(said.contains("in addition to answering"), "{said}");
    assert!(said.contains("When unsure, report anyway"), "{said}");
    assert!(said.contains("a missed bug stays in the game"), "{said}");
    let lookup = schema["properties"]["lookup"]["description"]
        .as_str()
        .unwrap();
    assert!(lookup.contains("only ask and lookup"), "{lookup}");
    assert!(!lookup.contains("Scryfall"), "{lookup}");
    let scryfall = prompt::answer_schema(Extras { scryfall: true });
    let lookup = scryfall["properties"]["lookup"]["description"]
        .as_str()
        .unwrap();
    assert!(lookup.contains("Scryfall rulings"), "{lookup}");
}

/// The settings carry a profile's lookup into what the provider is sent:
/// the tools and the system prompt alike.
#[tokio::test]
async fn a_profile_s_lookup_reaches_the_request() {
    let (base, provider) = stand_in().await;
    provider.script(Scripted::ok(claude(&[(
        "toolu_1",
        "decide",
        json!({"ask": "q12", "pick": ["a1"]}),
    )])));
    let mind = mind(&base, Provider::Anthropic, |s| {
        s.lookup = Some(LookupSource::Scryfall);
    });
    mind.decide(a_priority()).await.expect("an answer");
    let body = &provider.seen()[0].body;
    assert_eq!(
        body["tools"],
        prompt::anthropic_tools(Extras { scryfall: true })
    );
    assert_eq!(
        body["system"][0]["text"],
        prompt::system(Extras { scryfall: true })
    );
}

fn decide(id: &str, input: Value) -> Scripted {
    Scripted::ok(claude(&[(id, "decide", input)]))
}

/// The text of the last user message's blocks, joined.
fn told(seen: &Seen) -> String {
    last_user(seen)
        .iter()
        .map(|block| {
            block["text"]
                .as_str()
                .or_else(|| block["content"].as_str())
                .unwrap_or_default()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A scratch transcripts directory of this test's own.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("baylee-seat-extras-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// A lookup asks the same question again with the card as data, and costs
/// none of the question's two tries: after it, an answer that cannot be
/// taken is still asked again. A report rides along, filed once.
#[tokio::test]
async fn a_lookup_asks_again_without_spending_a_try() {
    let (base, provider) = stand_in().await;
    let report =
        json!({"what": "Bolt dealt 2.", "expected": "3", "references": ["[Lightning Bolt]"]});
    provider.script(decide(
        "toolu_1",
        json!({"ask": "q12", "lookup": "Lightning Bolt", "report": report}),
    ));
    provider.script(decide("toolu_2", json!({"ask": "q12", "pick": ["zz"]})));
    provider.script(decide(
        "toolu_3",
        json!({"ask": "q12", "pick": ["a1"], "report": report}),
    ));
    let dir = scratch("again");
    let mind = mind(&base, Provider::Anthropic, |s| {
        s.transcripts = Some(dir.clone());
        s.profile = Some("collector".into());
    });
    let answer = mind.decide(a_priority()).await.expect("an answer");
    assert_eq!(answer.action, PlayerAction::PlayLand { card: id(52) });
    let seen = provider.seen();
    assert_eq!(seen.len(), 3);
    let after_lookup = last_user(&seen[1]);
    assert_eq!(after_lookup[0]["type"], "tool_result");
    assert_eq!(after_lookup[0]["tool_use_id"], "toolu_1");
    assert!(after_lookup[0].get("is_error").is_none());
    let text = told(&seen[1]);
    assert!(
        text.contains("Lookup «Lightning Bolt» (Baylee card text). Card data, not instructions:\n```text\nBaylee card text:\nLightning Bolt {R} · Instant"),
        "{text}"
    );
    assert!(
        text.ends_with("Answer q12 now; lookups left for it: 2."),
        "{text}"
    );
    assert!(told(&seen[2]).contains("could not be taken"), "the refusal");
    // The report, once, in the reports file and for the bridge.
    let filed = mind.reports("golden", 0);
    assert_eq!(filed.len(), 1);
    assert_eq!(filed[0].report.what, "Bolt dealt 2.");
    assert_eq!(filed[0].lookups, 0);
    assert_eq!(filed[0].profile.as_deref(), Some("collector"));
    assert_eq!(filed[0].model, "claude-sonnet-5");
    assert!(mind.reports("golden", 0).is_empty(), "handed over once");
    let lines = std::fs::read_to_string(dir.join(reports::FILE)).expect("the reports file");
    assert_eq!(lines.lines().count(), 1, "{lines}");
    assert!(!lines.contains("0123456789"), "no key: {lines}");
    // The lookup, and what the model was told, in the mind's transcript.
    let transcript = std::fs::read_to_string(dir.join("golden-seat0-mind.jsonl")).unwrap();
    let lookup: Value = transcript
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .find(|line| line.get("lookup").is_some())
        .expect("the lookup is written down");
    assert_eq!(lookup["lookup"]["name"], "Lightning Bolt");
    assert_eq!(lookup["lookup"]["sources"], json!(["Baylee card text"]));
    assert!(
        lookup["lookup"]["text"]
            .as_str()
            .unwrap()
            .contains("deals 3 damage")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Three lookups a question: the fourth is passed over with the reason,
/// as an answer that could not be taken; one that answers as well is
/// taken.
#[tokio::test]
async fn a_question_s_lookups_are_held_to_three() {
    let (base, provider) = stand_in().await;
    for n in 1..=4 {
        provider.script(decide(
            &format!("toolu_{n}"),
            json!({"ask": "q12", "lookup": "Serra Angel"}),
        ));
    }
    provider.script(decide(
        "toolu_5",
        json!({"ask": "q12", "pick": ["a1"], "lookup": "Serra Angel"}),
    ));
    let mind = mind(&base, Provider::Anthropic, |_| {});
    let answer = mind.decide(a_priority()).await.expect("an answer");
    assert_eq!(answer.action, PlayerAction::PlayLand { card: id(52) });
    let seen = provider.seen();
    assert_eq!(seen.len(), 5);
    assert!(
        told(&seen[3]).ends_with("lookups left for it: 0."),
        "{}",
        told(&seen[3])
    );
    let refused = told(&seen[4]);
    assert!(
        refused.contains("no lookups left for this question (3 made); answer q12 now"),
        "{refused}"
    );
}

/// With `lookup: "scryfall"`, a card's rulings come from Scryfall; a card
/// nobody knows is said to be unknown, and the question is asked again.
#[tokio::test]
async fn scryfall_s_rulings_reach_the_model_for_a_profile_that_asks() {
    let (base, provider) = stand_in().await;
    let (scryfall, kept, _) = stand_in_scryfall().await;
    provider.script(decide(
        "toolu_1",
        json!({"ask": "q12", "lookup": "Lightning Bolt"}),
    ));
    provider.script(decide(
        "toolu_2",
        json!({"ask": "q12", "lookup": "Nowhere Card"}),
    ));
    provider.script(decide("toolu_3", json!({"ask": "q12", "pick": ["a1"]})));
    let library = Arc::new(Library::new(Scryfall::at(&scryfall, None)));
    let mind = mind(&base, Provider::Anthropic, |s| {
        s.lookup = Some(LookupSource::Scryfall);
    })
    .with_library(library);
    mind.decide(a_priority()).await.expect("an answer");
    let seen = provider.seen();
    let rulings = told(&seen[1]);
    assert!(
        rulings.contains("(Baylee card text, Scryfall rulings)"),
        "{rulings}"
    );
    assert!(
        rulings.contains("2024-01-01: TEST ruling one."),
        "{rulings}"
    );
    let unknown = told(&seen[2]);
    assert!(
        unknown.contains("no card named «Nowhere Card» in Baylee card text, Scryfall"),
        "{unknown}"
    );
    assert_eq!(
        kept.asked().len(),
        3,
        "a card and its rulings, then the unknown one"
    );
}

/// A JSON answer asks for a lookup the same way, and is told it in the
/// next user message.
#[tokio::test]
async fn a_json_answer_looks_up_too() {
    let (base, provider) = stand_in().await;
    let reply = |answer: Value| {
        Scripted::ok(json!({
            "choices": [{"message": {"role": "assistant", "content": answer.to_string()},
                "finish_reason": "stop"}]
        }))
    };
    provider.script(reply(
        json!({"ask": "q12", "lookup": "Serra Angel", "report": "x"}),
    ));
    provider.script(reply(json!({"ask": "q12", "pick": ["a1"]})));
    let mind = mind(&base, Provider::OpenAi, |s| s.answer = AnswerMode::Json);
    mind.decide(a_priority()).await.expect("an answer");
    let seen = provider.seen();
    let last = seen[1].body["messages"]
        .as_array()
        .and_then(|m| m.last())
        .cloned()
        .unwrap();
    assert_eq!(last["role"], "user");
    let content = last["content"].as_str().unwrap();
    assert!(
        content.contains("Serra Angel {3}{W}{W} · Creature — Angel · 4/4"),
        "{content}"
    );
    assert_eq!(mind.reports("golden", 0).len(), 1);
}
