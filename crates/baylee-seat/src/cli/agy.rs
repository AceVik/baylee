//! Google's Antigravity CLI, `agy`, as a mind's CLI: one process per
//! conversation, kept across turns as Claude Code's is ([`super`]), with
//! stream-json both ways, so each message is a line on its stdin and each
//! answer a `result` line on its stdout. A private `baylee-seat` agent
//! carries the instructions and enables only the output function `finish`.
//!
//! The agent is discovered under `.agents/agents` in the private working
//! directory. An empty tools list inherits tools, so it must explicitly
//! contain `finish`; `excludeDefaultComponents` drops agy's own prompt
//! sections and built-in tools (agy 1.5.18: a first model step read 470
//! input tokens where it read 11,029 without). Its `init` must confirm the
//! selected agent; agy still reports its global tool inventory there, not
//! the agent's filtered capabilities. agy offers every agent `manage_task`
//! beside `finish` (1.5.18); a step that uses it, or any tool but `finish`,
//! is a breach. Slash commands are disabled, no approval flag is passed, and
//! no MCP server may connect.
//! User rules under `~/.gemini` and the tool's own session storage remain
//! the CLI's (`docs/llm-seat.md` §"A CLI as the model").

use super::Brief;
use super::dialect::{Dialect, Event, Outcome, Started, Wire, clipped, listed, names};
use crate::llm::{Settings, Usage, json_object};
use baylee_client_core::llmseat::clis::agy_is_orchestrator;
use baylee_client_core::llmseat::{CliTool, DEFAULT_AGY_MODEL};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

/// Gemini CLI (Antigravity).
pub(crate) struct Agy;

/// An explicit capability list is essential: `tools: []` inherits all tools.
const AGENT: &str = "baylee-seat";
const AGENT_FILE: &str = ".agents/agents/baylee-seat.md";

impl Dialect for Agy {
    fn tool(&self) -> CliTool {
        CliTool::Agy
    }

    fn workspace_files(&self, brief: &Brief) -> Vec<(&'static str, String)> {
        vec![(
            AGENT_FILE,
            format!(
                "---\nname: {AGENT}\ndescription: Answer {task} in JSON.\n\
             tools: [finish]\nmainAgent: true\nsubagent: false\n\
             commandExecutionPolicy: off\nmcpServers: []\nskills: []\nplugins: []\n\
             excludeDefaultComponents: true\n---\n\
             {system}\n\n\
             Answer directly with {answer}. No commands, files, browser, MCP, \
             skills or other agents are needed or permitted.\n",
                task = brief.task,
                system = brief.system,
                answer = brief.answer,
            ),
        )]
    }

    fn args(
        &self,
        settings: &Settings,
        model: Option<&str>,
        brief: &Brief,
        _support: &Path,
    ) -> Vec<OsString> {
        let mut args: Vec<OsString> = [
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--disable-slash-commands",
            "--agent",
            AGENT,
        ]
        .into_iter()
        .map(OsString::from)
        .collect();
        args.push("--json-schema".into());
        args.push(brief.schema.to_string().into());
        let raw_model = model.unwrap_or(DEFAULT_AGY_MODEL);
        let (resolved_model, resolved_effort) =
            reconcile_agy_model_and_effort(raw_model, settings.effort.as_deref());
        args.push("--model".into());
        args.push(resolved_model.into());
        if let Some(effort) = resolved_effort {
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
        json!({
            "event": "user",
            "message": {
                "role": "user",
                "content": text,
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
                if init.get("agent").and_then(Value::as_str) != Some(AGENT) {
                    return Event::Breach(
                        "the agy process did not select the restricted baylee-seat agent: \
                         update agy before playing through it"
                            .into(),
                    );
                }
                Event::Started(started(init))
            }
            Some("step_update") => {
                let step = value.get("step_update").unwrap_or(&Value::Null);
                let kind = step.get("step_type").and_then(Value::as_str);
                // 1.5.18 names the tool at the step's top too (`tool_name`).
                let tool = step
                    .get("tool_info")
                    .and_then(|info| info.get("name"))
                    .or_else(|| step.get("tool_name"))
                    .and_then(Value::as_str);
                if kind == Some("tool") || tool.is_some() {
                    // `finish` only returns the answer; it has no external effect.
                    // The result line remains the single source of answer/usage.
                    if tool == Some("finish") {
                        return Event::Other;
                    }
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
        // 1.4.7 reports the global inventory even for our restricted agent.
        // Agent selection is checked before this; external tool use is
        // still a breach, and connected MCP servers are never accepted.
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
        // agy's read-only `/quota` in print mode (since 1.1.11): it asks the
        // signed-in account for its quota and calls no model.
        Some(
            ["-p", "/quota", "--output-format", "json"]
                .map(OsString::from)
                .into(),
        )
    }

    fn probe_ok(&self, stdout: &[u8], _stderr: &[u8]) -> bool {
        // Signed in where the account's quota came back: a `SUCCESS` that
        // names at least one group of models. Output it cannot read is no
        // login.
        let Ok(said) = serde_json::from_slice::<Value>(stdout) else {
            return false;
        };
        said.get("status").and_then(Value::as_str) == Some("SUCCESS")
            && said
                .pointer("/command/data/groups")
                .and_then(Value::as_array)
                .is_some_and(|groups| !groups.is_empty())
    }

    fn probe_wait(&self) -> Option<Duration> {
        // `/quota` asks the account over the network: five seconds on the
        // owner's Mac, 62 on the machine that plays the AI accounts
        // (27.11.2026).
        Some(QUOTA_WAIT)
    }

    fn probe_signed_out(&self, stdout: &[u8], stderr: &[u8]) -> bool {
        // Only an answer that names the login: a network error or a slow
        // server is no sign-out.
        let said = format!(
            "{} {}",
            String::from_utf8_lossy(stdout),
            String::from_utf8_lossy(stderr)
        )
        .to_lowercase();
        [
            "not signed in",
            "signed out",
            "sign in",
            "not logged in",
            "logged out",
            "log in",
            "login",
            "unauthenticated",
            "authentication",
        ]
        .iter()
        .any(|sign| said.contains(sign))
    }
}

/// How long agy's `/quota` login check may take.
pub(crate) const QUOTA_WAIT: Duration = Duration::from_secs(150);

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
    // Why a turn ended early (agy 1.4.11): `print_timeout`, `interrupted`
    // or `error`; absent or null for a turn that completed. A cut-off
    // response is never an answer.
    let cut_off = result
        .get("finished_reason")
        .and_then(Value::as_str)
        .filter(|reason| !reason.is_empty());
    let failed = status == "ERROR" || error_msg.is_some() || trouble.is_some() || cut_off.is_some();
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

    // A cut-off reply's partial response is not why it failed.
    let shown = error_msg.or(cut_off.is_none().then_some(said.as_str()));
    let words: String = shown.unwrap_or_default().chars().take(200).collect();
    let why = match (trouble, words.is_empty()) {
        (Some(kind), true) => kind.to_string(),
        (Some(kind), false) => format!("{kind}: {words}"),
        (None, true) => match cut_off {
            Some(reason) => format!("the reply ended early ({reason})"),
            None => format!("the reply failed ({status})"),
        },
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

/// The model families whose ids go up to an `-xhigh` tier (`agy models`,
/// 1.5.18); the others stop at `-high`.
const XHIGH_STEMS: [&str; 2] = ["gemini-4-hydrogen", "gemini-4-neon"];

/// Reconciles `agy`'s tier-suffixed model ids with `--effort` so `agy` never
/// rejects a conflicting `--model <stem>-<tier> --effort <other>` pair or
/// silently downgrades Gemini 4 Hydrogen's or Neon's `xhigh` tier to `high`.
/// An orchestrator takes no `--effort` at all (agy 1.5.18 refuses the game:
/// "--effort is not supported for model"): it plays the efforts agy's own
/// `orchestrator` setting names, and a profile's effort is not passed.
fn reconcile_agy_model_and_effort<'a>(
    raw_model: &str,
    effort: Option<&'a str>,
) -> (String, Option<&'a str>) {
    if agy_is_orchestrator(raw_model) {
        return (raw_model.to_string(), None);
    }
    let Some(stem) = agy_tier_stem(raw_model) else {
        return (raw_model.to_string(), effort);
    };
    let xhigh = XHIGH_STEMS.contains(&stem);
    match (stem, effort) {
        (_, None) => (raw_model.to_string(), None),
        ("gpt-oss-120b", Some(_)) => ("gpt-oss-120b-medium".to_string(), Some("medium")),
        (_, Some(e @ ("low" | "medium" | "high"))) => (format!("{stem}-{e}"), Some(e)),
        (_, Some("xhigh" | "max")) if xhigh => (format!("{stem}-xhigh"), Some("xhigh")),
        (_, Some("xhigh" | "max")) => (format!("{stem}-high"), Some("high")),
        (_, other) => (raw_model.to_string(), other),
    }
}

/// The stem of a tier-suffixed `agy` model id (`gemini-4-hydrogen-xhigh` →
/// `gemini-4-hydrogen`, `gemini-3.8-flash-high` → `gemini-3.8-flash`), or a
/// known bare tiered stem, else `None`.
fn agy_tier_stem(model: &str) -> Option<&str> {
    model
        .strip_suffix("-xhigh")
        .or_else(|| model.strip_suffix("-high"))
        .or_else(|| model.strip_suffix("-medium"))
        .or_else(|| model.strip_suffix("-low"))
        .or_else(|| {
            matches!(
                model,
                "gemini-4-hydrogen"
                    | "gemini-4-neon"
                    | "gemini-4-argon"
                    | "gemini-4-carbon"
                    | "gemini-3.8-flash"
                    | "claude-opus-5-5"
                    | "claude-sonnet-5-5"
                    | "gpt-oss-120b"
            )
            .then_some(model)
        })
}
