//! The Anthropic Messages API: `POST {base}/v1/messages`.
//!
//! Adaptive thinking with its summary shown (the terminal prints it), the
//! effort named, `tool_choice` left to the model (a forced tool is refused
//! beside thinking), and two cache points that live an hour: the system
//! prompt, and the deck prefix at the head of each turn's conversation.
//! Between two of a seat's turns the other players act, often for longer
//! than the five minutes of the default entry; an hour's entry costs twice
//! the input price to write against 1.25 times, and pays from the third
//! turn that reads it (`docs/llm-protocol.md` §"The cache"). The top-level
//! breakpoint carries the growing tail at five minutes, after the hour's
//! marks (the API takes the longer entries first). The model's content is
//! replayed byte for byte, thinking blocks and all; a conversation is only
//! ever appended to, and a fresh one starts each turn.

use super::prompt::{SYSTEM, anthropic_tools};
use super::{Call, Reply, Settings, Stop, ToolResult, Usage};
use serde_json::{Value, json};

/// The API version header this client speaks.
pub const VERSION: &str = "2023-06-01";

/// Where a message is posted.
#[must_use]
pub fn url(base: &str) -> String {
    format!("{base}/v1/messages")
}

/// Where readiness is asked: the model's own entry.
#[must_use]
pub fn model_url(base: &str, model: &str) -> String {
    format!("{base}/v1/models/{model}")
}

/// The cache mark of what every turn of a game repeats: an hour's entry.
#[must_use]
pub fn long_cache() -> Value {
    json!({"type": "ephemeral", "ttl": "1h"})
}

/// The request body for `messages`.
#[must_use]
pub fn body(settings: &Settings, messages: &[Value]) -> Value {
    let mut body = json!({
        "model": settings.model,
        "max_tokens": settings.max_tokens,
        "system": [{"type": "text", "text": SYSTEM, "cache_control": long_cache()}],
        "messages": messages,
        "tools": anthropic_tools(),
        "tool_choice": {"type": "auto"},
        "thinking": {"type": "adaptive", "display": "summarized"},
        "cache_control": {"type": "ephemeral"},
    });
    if let Some(effort) = &settings.effort {
        body["output_config"] = json!({"effort": effort});
    }
    body
}

/// Appends the user's turn: the results of the model's last tool calls
/// first (the API's order), then the conversation's prefix when it starts
/// one, cached for an hour, then the decision.
pub fn user(messages: &mut Vec<Value>, results: &[ToolResult], prefix: Option<&str>, text: &str) {
    let mut content: Vec<Value> = results
        .iter()
        .map(|result| {
            let mut block = json!({
                "type": "tool_result",
                "tool_use_id": result.id,
                "content": result.content,
            });
            if result.is_error {
                block["is_error"] = json!(true);
            }
            block
        })
        .collect();
    if let Some(prefix) = prefix {
        content.push(json!({"type": "text", "text": prefix, "cache_control": long_cache()}));
    }
    content.push(json!({"type": "text", "text": text}));
    messages.push(json!({"role": "user", "content": content}));
}

/// Appends a plain instruction to the user's turn that answers a reply
/// with no tool call.
pub fn nudge(messages: &mut Vec<Value>, results: &[ToolResult], text: &str) {
    user(messages, results, None, text);
}

/// Reads a response.
///
/// # Errors
/// When the body is not a Messages response.
pub fn parse(body: &Value) -> Result<Reply, String> {
    let content = body
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| "the response has no content".to_string())?;
    let mut text = String::new();
    let mut reasoning = String::new();
    let mut calls = Vec::new();
    for block in content {
        match block.get("type").and_then(Value::as_str) {
            Some("text") => {
                text.push_str(
                    block
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                );
            }
            Some("thinking") => {
                reasoning.push_str(
                    block
                        .get("thinking")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                );
            }
            Some("tool_use") => calls.push(Call {
                id: block
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                name: block
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                input: block.get("input").cloned().unwrap_or(Value::Null),
            }),
            _ => {}
        }
    }
    let usage = body
        .get("usage")
        .map_or_else(Usage::default, Usage::of_anthropic);
    let stop = match body.get("stop_reason").and_then(Value::as_str) {
        Some("end_turn" | "tool_use" | "stop_sequence" | "pause_turn") | None => Stop::Done,
        Some("refusal") => Stop::Refusal,
        Some("max_tokens") => Stop::MaxTokens,
        Some(other) => Stop::Other(other.to_string()),
    };
    Ok(Reply {
        // Replayed as it came: the thinking blocks' signatures bind them to
        // this conversation, and an edited block is dropped or refused.
        assistant: json!({"role": "assistant", "content": content}),
        calls,
        text: text.trim().to_string(),
        reasoning: (!reasoning.trim().is_empty()).then(|| reasoning.trim().to_string()),
        usage,
        stop,
    })
}

/// A provider's error body in one sentence.
#[must_use]
pub fn error_message(body: &Value) -> Option<String> {
    let error = body.get("error")?;
    let kind = error.get("type").and_then(Value::as_str).unwrap_or("error");
    let message = error.get("message").and_then(Value::as_str).unwrap_or("");
    Some(format!("{kind}: {message}"))
}
