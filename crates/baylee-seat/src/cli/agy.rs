//! Google's Antigravity CLI, `agy`, as a mind's CLI: one process per
//! conversation, kept across turns as Claude Code's is ([`super`]), with
//! stream-json both ways, so each message is a line on its stdin and each
//! answer a `result` line on its stdout. It takes no system prompt of ours
//! as a flag, so the first message of a conversation (the one that carries
//! the game's prefix) carries ours ahead of it, and every later one only
//! itself.
//!
//! Locked down less than Claude Code can be (`docs/llm-seat.md` §"A CLI as
//! the model"): `agy` has no flag that takes its tools, MCP servers or
//! rules files away. In print mode it approves nothing that asks (a
//! command, the web: "soft-denied"), and it is never given
//! `--dangerously-skip-permissions`; slash commands and skills are off
//! (`--disable-slash-commands`); its working directory is empty, so no
//! project's rules are read (its user rules under `~/.gemini` still are).
//! Its `init` line must name the tools it offers and must connect no MCP
//! server; and a step of the `tool` kind (the model used one, even one it
//! may) takes the mind off the table ([`Event::Breach`]). It keeps each
//! conversation in its own database: no flag turns that off.

use super::dialect::{Dialect, Event, Outcome, Started, Wire, clipped, listed, names};
use crate::cli::GAME_DATA;
use crate::llm::{Settings, Usage, json_object, prompt};
use baylee_client_core::llmseat::{CliTool, DEFAULT_AGY_MODEL};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::path::Path;

/// Gemini CLI (Antigravity).
pub(crate) struct Agy;

impl Dialect for Agy {
    fn tool(&self) -> CliTool {
        CliTool::Agy
    }

    fn args(
        &self,
        settings: &Settings,
        model: Option<&str>,
        _system: &str,
        _support: &Path,
    ) -> Vec<OsString> {
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

    fn read_event(&self, line: &str, wire: &mut Wire) -> Event {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return Event::Other;
        };
        let text = |key: &str| value.get(key).and_then(Value::as_str);
        match text("event") {
            Some("init") => {
                let init = value.get("init").unwrap_or(&value);
                Event::Started(started(init))
            }
            Some("step_update") => {
                let step = value.get("step_update").unwrap_or(&Value::Null);
                let kind = step.get("step_type").and_then(Value::as_str);
                let tool = step
                    .get("tool_info")
                    .and_then(|info| info.get("name"))
                    .and_then(Value::as_str);
                if kind == Some("tool") || tool.is_some() {
                    return Event::Breach(format!(
                        "the agy process let the model use {}: the seat does not play through it",
                        clipped(tool.unwrap_or("a tool"))
                    ));
                }
                Event::Other
            }
            Some("result") => {
                let result = value.get("result").unwrap_or(&value);
                Event::Reply(outcome(result, wire.trouble.take().as_deref()))
            }
            _ => Event::Other,
        }
    }

    fn lockdown_fault(&self, started: &Started) -> Option<String> {
        // Its tools cannot be taken away, so they are not judged here; a
        // tool used is ([`Event::Breach`]). That it named them, and that it
        // connected no MCP server where it says, is.
        if started.tools.is_none() {
            return Some(
                "the agy process did not say which tools it offers the model: the seat does not \
                 play through it"
                    .into(),
            );
        }
        match &started.mcp_servers {
            Some(servers) if !servers.is_empty() => Some(format!(
                "the agy process connected MCP servers ({}): the seat does not play through it",
                listed(servers)
            )),
            _ => None,
        }
    }

    fn probe_args(&self) -> Option<Vec<OsString>> {
        Some(["--version"].map(OsString::from).into())
    }

    fn probe_ok(&self, stdout: &[u8], _stderr: &[u8]) -> bool {
        !stdout.is_empty()
    }
}

/// What the `init` line names: the tools, the MCP servers, the slash
/// commands and the key's source.
fn started(init: &Value) -> Started {
    Started {
        tools: names(init, "tools"),
        mcp_servers: names(init, "mcp_servers"),
        slash_commands: names(init, "slash_commands"),
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
