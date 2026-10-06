//! The Junie CLI of `JetBrains`, `junie`, as a mind's CLI: one process per
//! message, non-interactive, the message as a JSON task on its stdin
//! (`--input-format=json`, `{"task": …}`) and its events as JSON lines on
//! its stdout (`--output-format=json-stream`).
//!
//! Junie takes one task a process and goes on only by `--resume` or
//! `--session-id`, which read a session it kept on disk; so every question
//! is a conversation of its own ([`Dialect::one_shot`]). Its ACP mode
//! (`--acp=true`) would hold a session, but runs Junie's own tools with
//! their permission prompts sent to the client and reports no reliable
//! token counts; it is not used.
//!
//! Locked down by flags (`docs/llm-seat.md` §"A CLI as the model"): no
//! user or project configuration (`--config-default-locations=false`, and
//! so none of its hooks), no MCP server, skill, command, custom agent or
//! custom model from their default places, no extensions (a directory of
//! the session's own, empty), no guidelines (`--guidelines-filename`
//! names an empty file of the session's own; the working directory is
//! empty besides), no update check, no statistics, its caches in the
//! session's directory, chat mode (`--agent-mode=chat`), and ours added
//! to its system prompt (`--system-prompt`, which Junie adds to its own:
//! it cannot be replaced). Brave mode, which approves actions, is never
//! asked for, and no key is ever passed (`--auth`, `--*-api-key`): Junie
//! plays on the login its owner signed in with.
//!
//! Junie names no tools at its start: its `session` line is the start,
//! and the proof the lockdown held is read off every line after it, so a
//! step other than the task's result (a file opened, a command run) takes
//! the mind off the table ([`Event::Breach`]). It still keeps each session
//! under its home (`~/.junie/sessions`): no flag turns that off.

use super::dialect::{Dialect, Event, Outcome, Started, Wire, clipped, sounds_limited};
use crate::llm::{Settings, Usage, json_object};
use baylee_client_core::llmseat::CliTool;
use serde_json::{Value, json};
use std::ffi::OsString;
use std::path::Path;

/// The guidelines file Junie is pointed at: empty.
const GUIDELINES: &str = "guidelines.md";

/// The only step a locked-down task takes: its result.
const RESULT_STEP: &str = "TASK RESULT";

/// The Junie CLI of `JetBrains`.
pub(crate) struct Junie;

impl Dialect for Junie {
    fn tool(&self) -> CliTool {
        CliTool::Junie
    }

    fn files(&self, _settings: &Settings, _system: &str) -> Vec<(&'static str, String)> {
        vec![(GUIDELINES, String::new())]
    }

    fn args(
        &self,
        settings: &Settings,
        model: Option<&str>,
        system: &str,
        support: &Path,
    ) -> Vec<OsString> {
        let path = |name: &str| support.join(name).to_string_lossy().into_owned();
        let mut args: Vec<String> = [
            "--input-format=json",
            "--output-format=json-stream",
            "--skip-update-check",
            "--share-anonymous-statistics=false",
            "--config-default-locations=false",
            "--mcp-default-locations=false",
            "--skill-default-locations=false",
            "--command-default-location=false",
            "--agent-default-location=false",
            "--model-default-locations=false",
            "--agent-mode=chat",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        args.push(format!(
            "--extensions-default-location={}",
            path("extensions")
        ));
        args.push(format!("--guidelines-filename={}", path(GUIDELINES)));
        args.push(format!("--cache-dir={}", path("cache")));
        args.push(format!("--system-prompt={system}"));
        if let Some(model) = model {
            args.push(format!("--model={model}"));
        }
        if let Some(effort) = &settings.effort {
            args.push(format!("--effort={effort}"));
        }
        args.into_iter().map(OsString::from).collect()
    }

    fn passed_env(&self) -> &'static [&'static str] {
        &["JUNIE_HOME"]
    }

    fn fixed_env(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }

    fn one_shot(&self) -> bool {
        true
    }

    fn stdin_line(&self, text: &str) -> String {
        json!({"task": text}).to_string()
    }

    fn usage_is_cumulative(&self) -> bool {
        // The `result` line counts the whole task, per model; one task a
        // process, so each reading is booked whole as a new process's
        // first.
        true
    }

    fn read_event(&self, line: &str, wire: &mut Wire) -> Event {
        // Its banner and anything else not JSON is nothing.
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return Event::Other;
        };
        let text = |key: &str| value.get(key).and_then(Value::as_str);
        match text("type") {
            Some("session") => Event::Started(Started::default()),
            Some("step") => {
                let name = text("name").unwrap_or("(unnamed)");
                if name == RESULT_STEP {
                    Event::Other
                } else {
                    Event::Breach(format!(
                        "the junie process took a step of its own ({}): the seat does not play \
                         through it",
                        clipped(name)
                    ))
                }
            }
            Some("error" | "system") => {
                if let Some(said) = ["message", "error", "details", "text"]
                    .iter()
                    .find_map(|key| text(key))
                {
                    wire.trouble = Some(said.to_string());
                }
                Event::Other
            }
            Some("result") => {
                let trouble = wire.trouble.take();
                Event::Reply(outcome(&value, trouble.as_deref()))
            }
            _ => Event::Other,
        }
    }

    fn lockdown_fault(&self, _started: &Started) -> Option<String> {
        // Its start names nothing; every later line is checked instead.
        None
    }

    fn probe_args(&self) -> Option<Vec<OsString>> {
        // Junie has no login check that calls no model: this one shows
        // only that the program runs.
        Some(vec!["--version".into()])
    }

    fn probe_ok(&self, stdout: &[u8], _stderr: &[u8]) -> bool {
        String::from_utf8_lossy(stdout).contains("Junie")
    }
}

/// A `result` line, read: a failure where it or an earlier line named
/// one, else the answer in its text.
fn outcome(result: &Value, trouble: Option<&str>) -> Outcome {
    let errors: Vec<&str> = result
        .get("errors")
        .and_then(Value::as_array)
        .map(|errors| errors.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let said = result
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if errors.is_empty() && trouble.is_none() {
        return Outcome::Answer {
            value: json_object(&said),
            text: said,
            usage: usage(result),
        };
    }
    let why = trouble
        .into_iter()
        .chain(errors)
        .collect::<Vec<_>>()
        .join("; ");
    let lower = why.to_lowercase();
    if sounds_limited(&why) || lower.contains("balance") || lower.contains("quota is exhausted") {
        Outcome::RateLimited {
            why: clipped(&why),
            lifts_in: None,
        }
    } else {
        Outcome::Failed(clipped(&why))
    }
}

/// The tokens a `result` counted: a list of per-model records found by
/// its shape (which key holds it has changed between builds), summed.
fn usage(result: &Value) -> Option<Usage> {
    let records = result.as_object()?.values().find_map(|value| {
        let list = value.as_array()?;
        (!list.is_empty() && list.iter().all(|item| item.get("inputTokens").is_some()))
            .then_some(list)
    })?;
    let mut total = Usage::default();
    for record in records {
        let n = |key: &str| record.get(key).and_then(Value::as_u64).unwrap_or(0);
        total.input += n("inputTokens");
        total.output += n("outputTokens");
        total.cache_read += n("cacheInputTokens");
        total.cache_write += n("cacheCreateTokens");
    }
    Some(total)
}
