//! Antigravity stream fixtures.

use super::*;

/// Claude Code's `init` line as agy says it.
pub(super) fn agy_init(line: &Value) -> Value {
    json!({"event": "init", "init": line})
}

/// Answers one message as agy would: only `answer` steps, with agy's
/// `cache_read_tokens` for Claude Code's `cache_read_input_tokens`.
pub(super) fn play_agy(step: &Value) {
    let kind = step.get("kind").and_then(Value::as_str).unwrap_or("answer");
    assert_eq!(kind, "answer", "the fake speaks only answers as agy");
    let answer = step.get("answer").cloned().unwrap_or(Value::Null);
    let usage = step.get("usage").cloned().unwrap_or_else(default_usage);
    let n = |key: &str| usage.get(key).and_then(Value::as_u64).unwrap_or(0);
    say(&json!({"event": "result", "result": {
        "status": "SUCCESS", "response": answer.to_string(), "structured_output": answer,
        "usage": {"input_tokens": n("input_tokens"), "output_tokens": n("output_tokens"),
                  "cache_read_tokens": n("cache_read_input_tokens")}}}));
}
