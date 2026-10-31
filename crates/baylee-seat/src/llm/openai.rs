//! An OpenAI-compatible chat endpoint: `POST {base}/chat/completions`.
//!
//! What `DeepSeek`, `OpenAI` and the local servers (llama.cpp, vLLM, Ollama)
//! share: messages with roles, function tools whose arguments arrive as a
//! JSON string, `role: tool` results. A model's `reasoning_content` is
//! retained verbatim in tool conversations: thinking endpoints such as
//! `DeepSeek` require it on subsequent calls. For a server without tools,
//! [`AnswerMode::Json`] asks for one
//! JSON object instead, and [`AnswerMode::JsonSchema`] for one held to the
//! answer's schema ([`answer_schema`]), which is what a server that refuses
//! a bare `json_object` takes (LM Studio: "`'response_format.type' must be
//! 'json_schema' or 'text'`").

use super::prompt::{Extras, JSON_MODE, answer_schema, openai_tools};
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

/// The conversation's first message: the system prompt for a seat with
/// `extras`.
#[must_use]
pub fn system(mode: AnswerMode, extras: Extras) -> Value {
    let mut text = super::prompt::system(extras);
    if mode.is_json() {
        text.push_str(JSON_MODE);
    }
    json!({"role": "system", "content": text})
}

/// The request body for the endpoint at `base`.
#[must_use]
pub fn body(settings: &Settings, base: &str, messages: &[Value]) -> Value {
    let mut body = json!({
        "model": settings.model,
        "messages": messages,
    });
    body[reply_limit(base)] = json!(settings.max_tokens);
    match settings.answer {
        AnswerMode::Tools => {
            body["tools"] = openai_tools(settings.extras());
            body["tool_choice"] = json!("auto");
        }
        AnswerMode::Json => body["response_format"] = json!({"type": "json_object"}),
        // Not `strict`: OpenAI's strict mode wants every field required and
        // no other allowed, and a decision fills only the fields its
        // question asks for.
        AnswerMode::JsonSchema => {
            body["response_format"] = json!({
                "type": "json_schema",
                "json_schema": {"name": "decide", "schema": answer_schema(settings.extras())},
            });
        }
    }
    if let Some(effort) = &settings.effort {
        body["reasoning_effort"] = json!(effort);
    }
    body
}

/// The field the endpoint at `base` takes the most a reply may take in.
/// `OpenAI`'s own API refuses `max_tokens` for its current models (`400
/// Unsupported parameter: 'max_tokens' is not supported with this model.
/// Use 'max_completion_tokens' instead.`, seen live on 31.10.2026), and
/// is told `max_completion_tokens`; every other OpenAI-compatible endpoint
/// (`DeepSeek`, Google's, a server on this machine) is told `max_tokens`,
/// which it takes.
#[must_use]
pub fn reply_limit(base: &str) -> &'static str {
    let host = base
        .split_once("://")
        .map_or(base, |(_, rest)| rest)
        .split(['/', ':', '?', '#'])
        .next()
        .unwrap_or_default();
    if host.eq_ignore_ascii_case("api.openai.com") {
        "max_completion_tokens"
    } else {
        "max_tokens"
    }
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
        // A synthetic call, decoded by the same JSON reader as a CLI.
        // The presence of an optional `concede` key is not a concession.
        calls.push(Call {
            id: String::new(),
            name: "decide".into(),
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
            cache_read: cached,
            ..Usage::default()
        }
    });
    let stop = match choice.get("finish_reason").and_then(Value::as_str) {
        Some("stop" | "tool_calls" | "function_call") | None => Stop::Done,
        Some("length") => Stop::MaxTokens,
        Some("content_filter") => Stop::Refusal,
        Some(other) => Stop::Other(other.to_string()),
    };
    let mut assistant = json!({"role": "assistant", "content": message.get("content").cloned().unwrap_or(Value::Null)});
    if !raw_calls.is_empty() {
        assistant["tool_calls"] = Value::Array(raw_calls);
    }
    // Preserve the provider's exact protocol field, including whitespace and
    // an empty string, independently of the trimmed text shown to a player.
    if mode == AnswerMode::Tools
        && let Some(reasoning) = message.get("reasoning_content").filter(|v| v.is_string())
    {
        assistant["reasoning_content"] = reasoning.clone();
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
