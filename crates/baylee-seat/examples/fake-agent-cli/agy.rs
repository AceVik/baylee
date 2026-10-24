//! Antigravity stream fixtures.

use super::{Value, default_usage, json, say};

/// Claude Code's `init` line as agy says it.
pub(super) fn agy_init(line: &Value) -> Value {
    let mut line = line.clone();
    let args: Vec<_> = std::env::args().collect();
    if args
        .windows(2)
        .any(|pair| pair == ["--agent", "baylee-seat"])
        && std::fs::read_to_string(".agents/agents/baylee-seat.md")
            .is_ok_and(|profile| profile.contains("tools: [finish]"))
    {
        line["agent"] = json!("baylee-seat");
    }
    json!({"event": "init", "init": line})
}

/// Answers one message as agy would: `answer` or `tool` steps, with agy's
/// `cache_read_tokens` for Claude Code's `cache_read_input_tokens`.
pub(super) fn play_agy(step: &Value) {
    let kind = step.get("kind").and_then(Value::as_str).unwrap_or("answer");
    assert!(matches!(kind, "answer" | "tool"), "unknown agy step");
    if kind == "tool" {
        say(&json!({"event": "step_update", "step_update": {
            "step_type": "tool", "state": "DONE",
            "tool_info": {"name": step.get("tool").unwrap_or(&json!("run_command"))}
        }}));
    }
    let answer = step.get("answer").cloned().unwrap_or(Value::Null);
    let usage = step.get("usage").cloned().unwrap_or_else(default_usage);
    let n = |key: &str| usage.get(key).and_then(Value::as_u64).unwrap_or(0);
    say(&json!({"event": "result", "result": {
        "status": "SUCCESS", "response": answer.to_string(), "structured_output": answer,
        "usage": {"input_tokens": n("input_tokens"), "output_tokens": n("output_tokens"),
                  "cache_read_tokens": n("cache_read_input_tokens")}}}));
}
