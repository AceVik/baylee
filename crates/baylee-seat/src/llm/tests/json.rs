//! OpenAI-compatible tools, JSON and schema responses.

use super::*;

#[test]
fn reasoning_replay_follows_the_answer_transport() {
    for mode in [AnswerMode::Tools, AnswerMode::Json, AnswerMode::JsonSchema] {
        let body = json!({"choices":[{"message":{
            "role":"assistant", "content":"", "reasoning_content":""
        }, "finish_reason":"stop"}]});
        let reply = openai::parse(&body, mode).unwrap();
        assert_eq!(
            reply.assistant.get("reasoning_content"),
            (mode == AnswerMode::Tools).then_some(&json!(""))
        );
    }
}

/// Optional JSON fields must not turn an ordinary action into a concession.
#[tokio::test]
async fn unused_concession_fields_do_not_concede() {
    for mode in [AnswerMode::Json, AnswerMode::JsonSchema] {
        for concede in [Value::Null, json!(""), json!("   ")] {
            let (base, provider) = stand_in().await;
            let answer = json!({"ask":"q12", "pick":["p"], "concede":concede});
            provider.script(Scripted::ok(json!({
                "choices":[{"message":{"role":"assistant", "content":answer.to_string()},
                    "finish_reason":"stop"}]
            })));
            let answer = mind(&base, Provider::OpenAi, |s| s.answer = mode)
                .decide(a_priority())
                .await
                .unwrap();
            assert_eq!(answer.action, PlayerAction::PassPriority);
        }
    }
}

/// A real concession carries the reason the model actually supplied.
#[tokio::test]
async fn json_concessions_keep_their_reason() {
    for mode in [AnswerMode::Json, AnswerMode::JsonSchema] {
        let (base, provider) = stand_in().await;
        provider.script(Scripted::ok(json!({
            "choices":[{"message":{"role":"assistant", "content":
                json!({"ask":"q12", "concede":"Lethal on board."}).to_string()},
                "finish_reason":"stop"}]
        })));
        let answer = mind(&base, Provider::OpenAi, |s| s.answer = mode)
            .decide(a_priority())
            .await
            .unwrap();
        assert_eq!(answer.action, PlayerAction::Concede);
        let note: Value = serde_json::from_str(answer.note.as_deref().unwrap()).unwrap();
        assert_eq!(note["say"], "Lethal on board.");
    }
}

/// Thinking tool endpoints require their reasoning bytes in the next request.
#[tokio::test]
async fn tool_conversations_preserve_reasoning_content() {
    let (base, provider) = stand_in().await;
    for question in [12, 13] {
        provider.script(Scripted::ok(json!({
            "choices":[{"message":{
                "role":"assistant", "content":null, "reasoning_content":"  Keep the land.\n",
                "tool_calls":[{"id":format!("call_{question}"), "type":"function", "function":{
                    "name":"decide", "arguments":json!({"ask":format!("q{question}"), "pick":["p"]}).to_string()
                }}]
            }, "finish_reason":"tool_calls"}]
        })));
    }
    let tools = mind(&base, Provider::OpenAi, |_| {});
    tools.decide(a_priority()).await.unwrap();
    let mut next = a_priority();
    next.question = 13;
    tools.decide(next).await.unwrap();
    let seen = provider.seen();
    let assistant = seen[1].body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|message| message["role"] == "assistant")
        .unwrap();
    assert_eq!(assistant["reasoning_content"], "  Keep the land.\n");
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
    // A model held to no schema is told the answer's shape on its question.
    let told = seen[1].body["messages"][1]["content"]
        .as_str()
        .expect("text");
    assert!(
        told.contains("Answer: ask=\"q12\", pick=[one option id]"),
        "{told}"
    );
}

/// The deck is told by name to an endpoint on this machine, which keeps no
/// prompt cache, and in full to Anthropic, which reads it back from one.
#[tokio::test]
async fn the_deck_is_told_by_name_where_nothing_caches_it() {
    let (base, provider) = stand_in().await;
    provider.script(Scripted::ok(json!({
        "choices": [{"message": {"role": "assistant",
            "content": "{\"ask\": \"q12\", \"pick\": [\"p\"]}"}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 900, "completion_tokens": 40}
    })));
    let local = mind(&base, Provider::OpenAi, |s| {
        s.answer = AnswerMode::JsonSchema;
    });
    local.decide(a_priority()).await.expect("an answer");
    let seen = provider.seen();
    let told = seen[0].body["messages"][1]["content"]
        .as_str()
        .expect("text");
    assert!(told.contains("YOUR DECK (40 cards), by name."), "{told}");
    assert!(told.contains("  Llanowar Elves {G} · Creature"), "{told}");
    assert!(!told.contains("Answer:"), "{told}");

    provider.script(Scripted::ok(claude(&[(
        "toolu_1",
        "decide",
        json!({"ask": "q12", "pick": ["p"]}),
    )])));
    let anthropic = mind(&base, Provider::Anthropic, |_| {});
    anthropic.decide(a_priority()).await.expect("an answer");
    let seen = provider.seen();
    let prefix = last_user(&seen[1])[0]["text"]
        .as_str()
        .expect("prefix")
        .to_string();
    assert!(
        prefix.contains("The full text of every card in it"),
        "{prefix}"
    );
}

/// An endpoint that takes no bare `json_object` (LM Studio) is asked for
/// an object held to the answer's schema, and is told the JSON rules in its
/// system prompt as the plain JSON mode is. An endpoint that turns a
/// `response_format` down says which mode to try instead.
#[tokio::test]
async fn an_openai_compatible_endpoint_answers_by_json_schema() {
    let (base, provider) = stand_in().await;
    provider.script(Scripted::ok(json!({
        "choices": [{"message": {"role": "assistant",
            "content": "{\"ask\": \"q12\", \"pick\": [\"a1\"], \"say\": \"Land.\"}"},
            "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 900, "completion_tokens": 40}
    })));
    let schema_mind = mind(&base, Provider::OpenAi, |s| {
        s.answer = AnswerMode::JsonSchema;
    });
    let answer = schema_mind
        .decide(a_priority())
        .await
        .expect("an answer by the schema");
    assert_eq!(answer.action, PlayerAction::PlayLand { card: id(52) });
    let seen = provider.seen();
    let format = &seen[0].body["response_format"];
    assert_eq!(format["type"], "json_schema");
    assert_eq!(format["json_schema"]["name"], "decide");
    assert_eq!(
        format["json_schema"]["schema"],
        prompt::answer_schema(prompt::Extras::default())
    );
    assert!(format["json_schema"].get("strict").is_none());
    let schema = prompt::answer_schema(prompt::Extras::default());
    for field in ["ask", "pick", "attacks", "then", "say", "concede"] {
        assert!(schema["properties"].get(field).is_some(), "{field}");
    }
    assert!(seen[0].body.get("tools").is_none());
    let system = seen[0].body["messages"][0]["content"].as_str().unwrap();
    assert!(system.ends_with(prompt::JSON_MODE), "{system}");

    // A concession by the schema is read as one.
    provider.script(Scripted::ok(json!({
        "choices": [{"message": {"role": "assistant",
            "content": "{\"ask\": \"q12\", \"concede\": \"Lethal on board.\"}"},
            "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 900, "completion_tokens": 40}
    })));
    let answer = mind(&base, Provider::OpenAi, |s| {
        s.answer = AnswerMode::JsonSchema;
    })
    .decide(a_priority())
    .await
    .expect("a concession");
    assert_eq!(answer.action, PlayerAction::Concede);

    // LM Studio's refusal of the other mode names the one to try.
    for (mode, other) in [
        (AnswerMode::Json, "answer: json_schema"),
        (AnswerMode::JsonSchema, "answer: json instead"),
    ] {
        provider.script(Scripted {
            status: 400,
            body: json!({"error": "'response_format.type' must be 'json_schema' or 'text'"}),
            delay: Duration::ZERO,
            headers: Vec::new(),
        });
        let refused = mind(&base, Provider::OpenAi, |s| {
            s.answer = mode;
        })
        .decide(a_priority())
        .await
        .expect_err("turned down");
        let MindError::Unavailable(why) = refused else {
            panic!("{refused:?}");
        };
        assert!(why.contains("400") && why.contains(other), "{why}");
    }
}
