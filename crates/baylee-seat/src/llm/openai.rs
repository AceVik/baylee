//! An OpenAI-compatible chat endpoint: `POST {base}/chat/completions`.
//!
//! What `DeepSeek`, `OpenAI` and the local servers (llama.cpp, vLLM, Ollama)
//! share: messages with roles, function tools whose arguments arrive as a
//! JSON string, `role: tool` results. A model's `reasoning_content` is
//! shown on the terminal and never sent back (`DeepSeek` refuses it in a
//! request). For a server without tools, [`AnswerMode::Json`] asks for one
//! JSON object instead, and [`AnswerMode::JsonSchema`] for one held to the
//! answer's schema ([`answer_schema`]), which is what a server that refuses
//! a bare `json_object` takes (LM Studio: "`'response_format.type' must be
//! 'json_schema' or 'text'`").

use super::prompt::{JSON_MODE, SYSTEM, answer_schema, openai_tools};
use super::{AnswerMode, Call, Reply, Settings, Stop, ToolResult, Usage};
use serde_json::{Value, json};

/// Where a message is posted.
#[must_use]
pub fn url(base: &str) -> String {
    format!("{base}/chat/completions")
}

/// Where readiness is asked.
#[must_use]
pub fn models_url(base: &str) -> String {
    format!("{base}/models")
}

/// The conversation's first message: the system prompt.
#[must_use]
pub fn system(mode: AnswerMode) -> Value {
    let text = if mode.is_json() {
        format!("{SYSTEM}{JSON_MODE}")
    } else {
        SYSTEM.to_string()
    };
    json!({"role": "system", "content": text})
}

/// The request body.
#[must_use]
pub fn body(settings: &Settings, messages: &[Value]) -> Value {
    let mut body = json!({
        "model": settings.model,
        "messages": messages,
        "max_tokens": settings.max_tokens,
    });
    match settings.answer {
        AnswerMode::Tools => {
            body["tools"] = openai_tools();
            body["tool_choice"] = json!("auto");
        }
        AnswerMode::Json => body["response_format"] = json!({"type": "json_object"}),
        // Not `strict`: OpenAI's strict mode wants every field required and
        // no other allowed, and a decision fills only the fields its
        // question asks for.
        AnswerMode::JsonSchema => {
            body["response_format"] = json!({
                "type": "json_schema",
                "json_schema": {"name": "decide", "schema": answer_schema()},
            });
        }
    }
    if let Some(effort) = &settings.effort {
        body["reasoning_effort"] = json!(effort);
    }
    body
}

/// Appends the user's turn: one `tool` message per result of the model's
/// last calls, then the decision (with the prefix ahead of it when it
/// starts a conversation).
pub fn user(messages: &mut Vec<Value>, results: &[ToolResult], prefix: Option<&str>, text: &str) {
    for result in results {
        let content = if result.is_error {
            format!("Error: {}", result.content)
        } else {
            result.content.clone()
        };
        messages.push(json!({"role": "tool", "tool_call_id": result.id, "content": content}));
    }
    let content = prefix.map_or_else(|| text.to_string(), |prefix| format!("{prefix}\n{text}"));
    messages.push(json!({"role": "user", "content": content}));
}

/// Reads a response.
///
/// # Errors
/// When the body is not a chat completion.
pub fn parse(body: &Value, mode: AnswerMode) -> Result<Reply, String> {
    let choice = body
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or_else(|| "the response has no choices".to_string())?;
    let message = choice
        .get("message")
        .ok_or_else(|| "the response's choice has no message".to_string())?;
    let text = message
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let reasoning = message
        .get("reasoning_content")
        .or_else(|| message.get("reasoning"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .map(str::to_string);
    let raw_calls = message
        .get("tool_calls")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut calls = Vec::new();
    for call in &raw_calls {
        let function = call.get("function").cloned().unwrap_or(Value::Null);
        let arguments = function.get("arguments");
        // The arguments are a JSON string; some servers send the object.
        let input = match arguments {
            Some(Value::String(s)) => serde_json::from_str(s).unwrap_or(Value::Null),
            Some(other) => other.clone(),
            None => Value::Null,
        };
        calls.push(Call {
            id: call
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            name: function
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            input,
        });
    }
    if mode.is_json()
        && calls.is_empty()
        && let Some(object) = json_object(&text)
    {
        let name = if object.get("concede").is_some() {
            "concede"
        } else {
            "decide"
        };
        calls.push(Call {
            id: String::new(),
            name: name.into(),
            input: object,
        });
    }
    let usage = body.get("usage").map_or_else(Usage::default, |u| {
        let n = |key: &str| u.get(key).and_then(Value::as_u64).unwrap_or(0);
        let cached = u
            .get("prompt_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or_else(|| n("prompt_cache_hit_tokens"));
        Usage {
            input: n("prompt_tokens").saturating_sub(cached),
            output: n("completion_tokens"),
            cache_write: 0,
            cache_read: cached,
        }
    });
    let stop = match choice.get("finish_reason").and_then(Value::as_str) {
        Some("stop" | "tool_calls" | "function_call") | None => Stop::Done,
        Some("length") => Stop::MaxTokens,
        Some("content_filter") => Stop::Refusal,
        Some(other) => Stop::Other(other.to_string()),
    };
    // Replayed without the reasoning, which such endpoints refuse to be
    // sent back.
    let mut assistant = json!({"role": "assistant", "content": message.get("content").cloned().unwrap_or(Value::Null)});
    if !raw_calls.is_empty() {
        assistant["tool_calls"] = Value::Array(raw_calls);
    }
    Ok(Reply {
        assistant,
        calls,
        text,
        reasoning,
        usage,
        stop,
    })
}

/// The first JSON object in `text`: the whole text, or the text inside a
/// code fence, or the outermost `{…}`.
#[must_use]
pub fn json_object(text: &str) -> Option<Value> {
    if let Ok(value @ Value::Object(_)) = serde_json::from_str::<Value>(text) {
        return Some(value);
    }
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    (start < end)
        .then(|| serde_json::from_str::<Value>(&text[start..=end]).ok())
        .flatten()
        .filter(Value::is_object)
}

/// What to try when an endpoint turned the request's `response_format`
/// down: the other JSON mode, or none for a request that sent none.
#[must_use]
pub fn response_format_hint(why: &str, mode: AnswerMode) -> Option<&'static str> {
    if !why.contains("response_format") {
        return None;
    }
    match mode {
        AnswerMode::Json => Some("the endpoint may take answer: json_schema instead"),
        AnswerMode::JsonSchema => Some("the endpoint may take answer: json instead"),
        AnswerMode::Tools => None,
    }
}

/// A provider's error body in one sentence.
#[must_use]
pub fn error_message(body: &Value) -> Option<String> {
    let error = body.get("error")?;
    if let Some(message) = error.as_str() {
        return Some(message.to_string());
    }
    let message = error.get("message").and_then(Value::as_str).unwrap_or("");
    let kind = error
        .get("type")
        .or_else(|| error.get("code"))
        .and_then(Value::as_str)
        .unwrap_or("error");
    Some(format!("{kind}: {message}"))
}
