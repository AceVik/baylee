//! Gemini CLI, `agy`, as a mind's CLI: one process per conversation, kept
//! across turns as Claude Code's is ([`super`]), with stream-json both
//! ways, so each message is a line on its stdin and each answer a `result`
//! line on its stdout. It takes no system prompt of ours as a flag, so the
//! first message of a conversation (the one that carries the game's
//! prefix) carries ours ahead of it, and every later one only itself.

use super::dialect::{Dialect, Event, Outcome, Started};
use crate::cli::GAME_DATA;
use crate::llm::{Settings, Usage, json_object, prompt};
use baylee_client_core::llmseat::{CliTool, DEFAULT_AGY_MODEL};
use serde_json::{Value, json};
use std::ffi::OsString;

/// Gemini CLI (Antigravity).
pub(crate) struct Agy;

impl Dialect for Agy {
    fn tool(&self) -> CliTool {
        CliTool::Agy
    }

    fn args(&self, settings: &Settings, model: Option<&str>, _system: &str) -> Vec<OsString> {
        let mut args: Vec<OsString> = [
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--disable-slash-commands",
        ]
        .into_iter()
        .map(OsString::from)
        .collect();
        args.push("--json-schema".into());
        args.push(prompt::answer_schema().to_string().into());
        args.push("--model".into());
        args.push(model.unwrap_or(DEFAULT_AGY_MODEL).into());
        if let Some(effort) = &settings.effort {
            args.push("--effort".into());
            args.push(effort.into());
        }
        args
    }

    fn passed_env(&self) -> &'static [&'static str] {
        &[]
    }

    fn fixed_env(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }

    fn stdin_line(&self, text: &str) -> String {
        let content = if text.contains("THE GAME\nYou are") {
            format!(
                "{}{}{GAME_DATA}\n\n{text}",
                prompt::SYSTEM,
                prompt::JSON_MODE
            )
        } else {
            text.to_string()
        };
        json!({
            "event": "user",
            "message": {
                "role": "user",
                "content": content,
            }
        })
        .to_string()
    }

    fn usage_is_cumulative(&self) -> bool {
        // Read off recorded games: a process's `input_tokens` and
        // `cache_read_tokens` only grow from reply to reply and begin again
        // with a new process (one game booked 19.7 M tokens summed, about
        // 3.2 M as differences).
        true
    }

    fn read_event(&self, line: &str, trouble: &mut Option<String>) -> Event {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return Event::Other;
        };
        let text = |key: &str| value.get(key).and_then(Value::as_str);
        match text("event") {
            Some("init") => {
                let init = value.get("init").unwrap_or(&value);
                Event::Started(started(init))
            }
            Some("result") => {
                let result = value.get("result").unwrap_or(&value);
                Event::Reply(outcome(result, trouble.take().as_deref()))
            }
            _ => Event::Other,
        }
    }

    fn lockdown_fault(&self, _started: &Started) -> Option<String> {
        None
    }

    fn probe_args(&self) -> Option<Vec<OsString>> {
        Some(["--version"].map(OsString::from).into())
    }

    fn probe_ok(&self, stdout: &[u8]) -> bool {
        !stdout.is_empty()
    }
}

/// What the `init` line names: the tools, the MCP servers, the slash
/// commands and the key's source.
fn started(init: &Value) -> Started {
    let names = |key: &str| -> Option<Vec<String>> {
        let items = init.get(key)?.as_array()?;
        Some(
            items
                .iter()
                .map(|item| {
                    item.as_str()
                        .or_else(|| item.get("name").and_then(Value::as_str))
                        .unwrap_or("(unnamed)")
                        .to_string()
                })
                .collect(),
        )
    };
    Started {
        tools: names("tools"),
        mcp_servers: names("mcp_servers"),
        slash_commands: names("slash_commands"),
        key_source: init
            .get("apiKeySource")
            .and_then(Value::as_str)
            .map(str::to_string),
    }
}

/// A `result` line, read: an answer, a rate limit, or a failure.
fn outcome(result: &Value, trouble: Option<&str>) -> Outcome {
    let status = result
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let error_msg = result.get("error").and_then(Value::as_str);
    let failed = status == "ERROR" || error_msg.is_some() || trouble.is_some();
    let said = result
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    if !failed {
        let value = result
            .get("structured_output")
            .filter(|v| v.is_object())
            .cloned()
            .or_else(|| json_object(&said));
        return Outcome::Answer {
            value,
            text: said,
            usage: result.get("usage").map(usage),
        };
    }

    let words: String = error_msg.unwrap_or(&said).chars().take(200).collect();
    let why = match (trouble, words.is_empty()) {
        (Some(kind), true) => kind.to_string(),
        (Some(kind), false) => format!("{kind}: {words}"),
        (None, true) => format!("the reply failed ({status})"),
        (None, false) => words,
    };
    let lower = why.to_lowercase();
    let limited = [
        "rate limit",
        "usage limit",
        "quota",
        "429",
        "resource_exhausted",
    ]
    .iter()
    .any(|sign| lower.contains(sign));
    if limited {
        return Outcome::RateLimited {
            why,
            lifts_in: None,
        };
    }
    Outcome::Failed(why)
}

/// The tokens a `result` counted.
fn usage(usage: &Value) -> Usage {
    let n = |key: &str| usage.get(key).and_then(Value::as_u64).unwrap_or(0);
    Usage {
        input: n("input_tokens"),
        output: n("output_tokens"),
        cache_read: n("cache_read_tokens"),
        ..Usage::default()
    }
}
