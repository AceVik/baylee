//! Claude Code, `claude`, as a mind's CLI: one process per conversation,
//! in print mode with stream-json both ways, so each message is a line on
//! its stdin and each answer a `result` line on its stdout.
//!
//! The process is locked down by its flags (`docs/llm-seat.md` §"A CLI as
//! the model"): no tools (`--tools ""`), no MCP server
//! (`--strict-mcp-config` with none named), no skill or slash command, no
//! user, project or local settings and so none of their hooks
//! (`--restricted`, `--setting-sources ""`), no `CLAUDE.md`, plugin or
//! custom agent (`--safe-mode`), nothing that would ask for a permission
//! (`--permission-prompts none`), nothing kept on disk
//! (`--no-session-persistence`), our instructions in place of its own
//! (`--system-prompt`) and our answer's schema (`--json-schema`). `--bare`
//! would lock it down further and is not used: it never reads the login a
//! subscription plays with.
//!
//! Its `init` line is the proof the flags held, checked before any reply
//! is taken: it must name its tools (none but the answer's own), its MCP
//! servers and its slash commands (none of either), and its key's source
//! must not be a variable (`ANTHROPIC_API_KEY`); a subscription's is
//! `none`.

use super::dialect::{Dialect, Event, Outcome, Started};
use crate::llm::{Settings, Usage, json_object, prompt};
use baylee_client_core::llmseat::CliTool;
use serde_json::{Value, json};
use std::ffi::OsString;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The only tool the model may be offered: the one the tool itself adds to
/// hand back an answer held to `--json-schema`.
const ANSWER_TOOL: &str = "StructuredOutput";

/// Claude Code.
pub(crate) struct Claude;

impl Dialect for Claude {
    fn tool(&self) -> CliTool {
        CliTool::Claude
    }

    fn args(&self, settings: &Settings, model: Option<&str>, system: &str) -> Vec<OsString> {
        let mut args: Vec<OsString> = [
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--restricted",
            "--safe-mode",
            "--tools",
            "",
            "--strict-mcp-config",
            "--disable-slash-commands",
            "--setting-sources",
            "",
            "--permission-prompts",
            "none",
            "--permission-mode",
            "manual",
            "--no-session-persistence",
            "--system-prompt",
        ]
        .into_iter()
        .map(OsString::from)
        .collect();
        args.push(system.into());
        args.push("--json-schema".into());
        args.push(prompt::answer_schema().to_string().into());
        if let Some(model) = model {
            args.push("--model".into());
            args.push(model.into());
        }
        if let Some(effort) = &settings.effort {
            args.push("--effort".into());
            args.push(effort.into());
        }
        args
    }

    fn passed_env(&self) -> &'static [&'static str] {
        &["CLAUDE_CONFIG_DIR"]
    }

    fn fixed_env(&self) -> &'static [(&'static str, &'static str)] {
        // The program is a version's own file, and stays that version: a
        // game's process never updates it underneath the next one.
        &[("DISABLE_AUTOUPDATER", "1")]
    }

    fn stdin_line(&self, text: &str) -> String {
        json!({"type": "user", "message": {"role": "user", "content": text}}).to_string()
    }

    fn read_event(&self, line: &str, trouble: &mut Option<String>) -> Event {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return Event::Other;
        };
        let text = |key: &str| value.get(key).and_then(Value::as_str);
        match text("type") {
            Some("system") if text("subtype") == Some("init") => Event::Started(started(&value)),
            Some("assistant") => {
                if let Some(error) = text("error") {
                    *trouble = Some(error.to_string());
                }
                Event::Other
            }
            Some("result") => Event::Reply(outcome(&value, trouble.take().as_deref())),
            _ => Event::Other,
        }
    }

    fn lockdown_fault(&self, started: &Started) -> Option<String> {
        let refused = |what: String| {
            Some(format!(
                "the claude process {what}: the seat does not play through it"
            ))
        };
        let Some(tools) = &started.tools else {
            return refused("did not say which tools it offers the model".into());
        };
        let tools: Vec<String> = tools
            .iter()
            .filter(|tool| *tool != ANSWER_TOOL)
            .cloned()
            .collect();
        if !tools.is_empty() {
            return refused(format!(
                "offered the model tools it must not have ({})",
                listed(&tools)
            ));
        }
        let Some(servers) = &started.mcp_servers else {
            return refused("did not say which MCP servers it connected".into());
        };
        if !servers.is_empty() {
            return refused(format!("connected MCP servers ({})", listed(servers)));
        }
        let Some(commands) = &started.slash_commands else {
            return refused("did not say which slash commands it offers".into());
        };
        if !commands.is_empty() {
            return refused(format!("offered slash commands ({})", listed(commands)));
        }
        match started.key_source.as_deref() {
            Some(source) if names_a_variable(source) => {
                refused(format!("calls the model with a key from {source}"))
            }
            _ => None,
        }
    }

    fn probe_args(&self) -> Option<Vec<OsString>> {
        Some(["auth", "status", "--json"].map(OsString::from).into())
    }

    fn probe_ok(&self, stdout: &[u8]) -> bool {
        // Signed in only where the status says so: output it cannot read
        // is no login.
        serde_json::from_slice::<Value>(stdout)
            .ok()
            .and_then(|status| status.get("loggedIn").and_then(Value::as_bool))
            .unwrap_or(false)
    }
}

/// What the `init` line names: the tools, the MCP servers, the slash
/// commands and the key's source. An entry that is neither a name nor
/// has one is kept, as `(unnamed)`, so it is never dropped unseen.
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

/// Up to eight names, for a sentence.
fn listed(names: &[String]) -> String {
    let mut shown = names.iter().take(8).cloned().collect::<Vec<_>>().join(", ");
    if names.len() > 8 {
        shown.push_str(", …");
    }
    shown
}

/// Whether a key's source is an environment variable's name
/// (`ANTHROPIC_API_KEY`), not a login (`none`, `/login managed key`).
fn names_a_variable(source: &str) -> bool {
    source.contains('_')
        && source
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// A `result` line, read: an answer, a rate limit, or a failure. `trouble`
/// is the error an `assistant` line of the same reply named.
fn outcome(result: &Value, trouble: Option<&str>) -> Outcome {
    let said = result
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let subtype = result
        .get("subtype")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let failed = result
        .get("is_error")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || subtype.starts_with("error");
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
    let words: String = said.chars().take(200).collect();
    let why = match (trouble, words.is_empty()) {
        (Some(kind), true) => kind.to_string(),
        (Some(kind), false) => format!("{kind}: {words}"),
        (None, true) => format!("the reply failed ({subtype})"),
        (None, false) => words.clone(),
    };
    let lower = said.to_lowercase();
    let limited = matches!(trouble, Some("rate_limit" | "billing_error"))
        || ["rate limit", "usage limit", "quota", "429"]
            .iter()
            .any(|sign| lower.contains(sign));
    if limited {
        return Outcome::RateLimited {
            why: why.split('|').next().unwrap_or_default().trim().to_string(),
            lifts_in: lifts_in(&said),
        };
    }
    if trouble == Some("authentication_failed") {
        return Outcome::Failed(format!(
            "claude is not signed in ({why}): sign in once with claude itself"
        ));
    }
    Outcome::Failed(why)
}

/// When a limit lifts, where the reply says: a Unix time after the last
/// `|`, as Claude Code's usage-limit text ends.
fn lifts_in(said: &str) -> Option<Duration> {
    let (_, at) = said.rsplit_once('|')?;
    let at = Duration::from_secs(at.trim().parse::<u64>().ok()?);
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    Some(at.saturating_sub(now))
}

/// The tokens a `result` counted. Every read of the context counts,
/// cache reads included, as for an API.
fn usage(usage: &Value) -> Usage {
    let n = |key: &str| usage.get(key).and_then(Value::as_u64).unwrap_or(0);
    Usage {
        input: n("input_tokens"),
        output: n("output_tokens"),
        cache_write: n("cache_creation_input_tokens"),
        cache_read: n("cache_read_input_tokens"),
    }
}
