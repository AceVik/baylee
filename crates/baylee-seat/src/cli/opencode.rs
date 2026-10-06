//! opencode, `opencode`, as a mind's CLI: `opencode run`, one process per
//! message, the message on its stdin (read to its end; nothing in argv,
//! which `run` would requote) and its events as JSON lines on its stdout
//! (`--format json`).
//!
//! `opencode run` answers one message and ends ([`Dialect::one_shot`]),
//! and goes on with a conversation by its id (`--session <id>`, the
//! `sessionID` every line names; never `--continue`, the most recent one),
//! read back from its database ([`Dialect::resumes`]). That database is a
//! file in the seat's own store (`OPENCODE_DB`, an absolute path), not the
//! user's, and goes with the store; the login stays where it is
//! (`auth.json` under the data directory). A session it does not find is
//! `Session not found` on stderr and an exit before any line: the mind
//! begins the conversation again. Its ACP mode (`opencode acp`) would hold
//! a session in one process; it is not used yet.
//!
//! Locked down by its environment and a configuration file of the
//! session's own (`docs/llm-seat.md` §"A CLI as the model"): every tool
//! denied, which drops it from what the model is offered (`permission`
//! `{"*": "deny"}`, on the agent too); no MCP server (`mcp` empty, and the
//! user's own configuration directory replaced by an empty one, so none of
//! its servers, plugins or `AGENTS.md` are read); no external plugin
//! (`--pure`); no project configuration or instruction file
//! (`OPENCODE_DISABLE_PROJECT_CONFIG`), none of Claude Code's
//! (`OPENCODE_DISABLE_CLAUDE_CODE`), no external skill; a database in the
//! seat's store, so no session reaches the user's; no update, share,
//! snapshot, formatter or compaction; our instructions in place of the
//! provider prompt (an agent `seat` whose `prompt` is ours: opencode still
//! adds its own environment block, naming the model, the working directory
//! and the date); and a title given (`--title`), so no model call is made
//! to write one (`--title` is ignored on a resumed session, which has one).
//! The configuration, `--agent` and `--model` are given again to every
//! process, a resumed one too, so a resumed session is as locked down as a
//! new one. `--auto`, which approves what is not denied, is never passed:
//! `run` rejects every permission request without it.
//!
//! opencode says nothing at its start: its first line (whatever it says;
//! each names the session) is the start, and the proof the lockdown held is
//! read off every line after it, so a `tool_use` line takes the mind off
//! the table ([`Event::Breach`]). A reply ends at its `step_finish`, whose
//! tokens are that step's own.

use super::dialect::{
    Dialect, Event, Outcome, Started, Wire, clipped, conversation_id, first_line_starts,
    sounds_limited,
};
use crate::llm::{Settings, Usage, json_object};
use baylee_client_core::llmseat::CliTool;
use serde_json::{Value, json};
use std::ffi::OsString;
use std::path::Path;

/// The configuration file opencode is pointed at.
const CONFIG: &str = "opencode.json";

/// The configuration directory opencode is given in place of the user's:
/// empty.
const CONFIG_HOME: &str = "config";

/// The database file in the seat's store.
const DATABASE: &str = "opencode.db";

/// The agent the seat plays as, defined in [`CONFIG`].
const AGENT: &str = "seat";

/// opencode.
pub(crate) struct Opencode;

impl Dialect for Opencode {
    fn tool(&self) -> CliTool {
        CliTool::Opencode
    }

    fn files(&self, _settings: &Settings, system: &str) -> Vec<(&'static str, String)> {
        let config = json!({
            "$schema": "https://opencode.ai/config.json",
            "autoupdate": false,
            "share": "disabled",
            "snapshot": false,
            "formatter": false,
            "compaction": {"auto": false},
            "mcp": {},
            "instructions": [],
            "permission": {"*": "deny"},
            "agent": {
                AGENT: {
                    "mode": "primary",
                    "prompt": system,
                    "permission": {"*": "deny"},
                },
            },
        });
        vec![(CONFIG, config.to_string())]
    }

    fn args(
        &self,
        settings: &Settings,
        model: Option<&str>,
        _system: &str,
        _support: &Path,
    ) -> Vec<OsString> {
        let mut args: Vec<OsString> = [
            "run", "--pure", "--format", "json", "--agent", AGENT, "--title", AGENT,
        ]
        .into_iter()
        .map(OsString::from)
        .collect();
        if let Some(model) = model {
            args.push("--model".into());
            args.push(model.into());
        }
        if let Some(effort) = &settings.effort {
            args.push("--variant".into());
            args.push(effort.into());
        }
        args
    }

    fn passed_env(&self) -> &'static [&'static str] {
        // Where its login (`auth.json` under the data directory), its
        // cache and its logs live, where the parent moved them.
        &["XDG_DATA_HOME", "XDG_CACHE_HOME", "XDG_STATE_HOME"]
    }

    fn fixed_env(&self) -> &'static [(&'static str, &'static str)] {
        &[
            ("OPENCODE_DISABLE_PROJECT_CONFIG", "1"),
            ("OPENCODE_DISABLE_CLAUDE_CODE", "1"),
            ("OPENCODE_DISABLE_EXTERNAL_SKILLS", "1"),
            ("OPENCODE_DISABLE_AUTOUPDATE", "1"),
            ("OPENCODE_DISABLE_AUTOCOMPACT", "1"),
            ("OPENCODE_DISABLE_LSP_DOWNLOAD", "1"),
        ]
    }

    fn session_env(&self, support: &Path) -> Vec<(&'static str, OsString)> {
        vec![
            ("OPENCODE_CONFIG", support.join(CONFIG).into()),
            ("XDG_CONFIG_HOME", support.join(CONFIG_HOME).into()),
        ]
    }

    fn one_shot(&self) -> bool {
        true
    }

    fn resumes(&self) -> bool {
        true
    }

    fn store_env(&self, store: &Path) -> Vec<(&'static str, OsString)> {
        // An absolute path is taken as it is (a relative one would be under
        // the user's data directory); its `-wal` and `-shm` beside it.
        vec![("OPENCODE_DB", store.join(DATABASE).into())]
    }

    fn resume(&self, args: &mut Vec<OsString>, id: &str) {
        args.extend(["--session".into(), id.into()]);
    }

    fn stdin_line(&self, text: &str) -> String {
        // The whole of stdin is the message, as it stands.
        text.to_string()
    }

    fn usage_is_cumulative(&self) -> bool {
        // A `step_finish` counts its own step (opencode's processor writes
        // each step's usage into its part).
        false
    }

    fn read_event(&self, line: &str, wire: &mut Wire) -> Event {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return Event::Other;
        };
        if wire.conversation.is_none() {
            wire.conversation = value
                .get("sessionID")
                .and_then(Value::as_str)
                .and_then(conversation_id);
        }
        if let Some(start) = first_line_starts(&value, wire) {
            return start;
        }
        let Some(kind) = value.get("type").and_then(Value::as_str) else {
            return Event::Other;
        };
        let part = value.get("part").unwrap_or(&Value::Null);
        match kind {
            "tool_use" => {
                let tool = part.get("tool").and_then(Value::as_str).unwrap_or("a tool");
                Event::Breach(format!(
                    "the opencode process let the model use {}: the seat does not play through \
                     it",
                    clipped(tool)
                ))
            }
            "text" => {
                if let Some(text) = part.get("text").and_then(Value::as_str) {
                    wire.said.get_or_insert_with(String::new).push_str(text);
                }
                Event::Other
            }
            "step_finish" => {
                let said = wire.said.take().unwrap_or_default();
                Event::Reply(Outcome::Answer {
                    value: json_object(&said),
                    text: said,
                    usage: part.get("tokens").map(usage),
                })
            }
            "error" => {
                wire.said = None;
                Event::Reply(failure(value.get("error").unwrap_or(&Value::Null)))
            }
            _ => Event::Other,
        }
    }

    fn lockdown_fault(&self, _started: &Started) -> Option<String> {
        // Its start names nothing; every later line is checked instead.
        None
    }

    fn probe_args(&self) -> Option<Vec<OsString>> {
        // Lists its stored credentials; calls no model.
        Some(["auth", "list"].map(OsString::from).into())
    }

    fn probe_ok(&self, stdout: &[u8], _stderr: &[u8]) -> bool {
        // A subscription's credential is a sign-in (`oauth`); a stored key
        // (`api`) is not one.
        String::from_utf8_lossy(stdout)
            .lines()
            .any(|line| line.split_whitespace().last() == Some("oauth"))
    }
}

/// An `error` line's error: a rate limit by its status or its words, a
/// sign-in that is missing, or a failure.
fn failure(error: &Value) -> Outcome {
    let name = error.get("name").and_then(Value::as_str).unwrap_or("error");
    let data = error.get("data").unwrap_or(&Value::Null);
    let message = data
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let why = if message.is_empty() {
        name.to_string()
    } else {
        format!("{name}: {message}")
    };
    let status = data.get("statusCode").and_then(Value::as_u64);
    if status == Some(429) || sounds_limited(&why) {
        return Outcome::RateLimited {
            why: clipped(&why),
            lifts_in: None,
        };
    }
    if name == "ProviderAuthError" {
        return Outcome::Failed(format!(
            "opencode is not signed in ({}): sign in once with opencode itself",
            clipped(&why)
        ));
    }
    Outcome::Failed(clipped(&why))
}

/// A step's tokens: its `input` is without the cache's, its `output`
/// without the reasoning, which is output too.
fn usage(tokens: &Value) -> Usage {
    let n = |value: &Value, key: &str| value.get(key).and_then(Value::as_u64).unwrap_or(0);
    let cache = tokens.get("cache").unwrap_or(&Value::Null);
    Usage {
        input: n(tokens, "input"),
        output: n(tokens, "output") + n(tokens, "reasoning"),
        cache_read: n(cache, "read"),
        cache_write: n(cache, "write"),
        ..Usage::default()
    }
}
