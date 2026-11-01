//! The Junie CLI of `JetBrains`, `junie`, as a mind's CLI: one process per
//! conversation in its ACP mode (`--acp=true`), the Agent Client Protocol
//! on its stdin and stdout ([`super::acp`]), one session held across the
//! game's turns, as Claude Code's process holds its conversation.
//!
//! Junie is a coding agent and keeps its tools whatever its flags say:
//! `--agent-mode=chat`, which would take them away, is refused by the
//! released builds (26.10.27: "not available in this version"), and its
//! non-interactive mode approves every terminal command itself (measured
//! 31.10.2026: `touch` and `ls -la ~/.junie` ran with its allowlist set to
//! ask, and so did a game's `echo` and `ls`). In ACP mode with brave mode
//! off it asks its client before each command instead, even `ls`, and the
//! seat always refuses: a refused permission is counted and written to the
//! transcript, not a breach, and the model is told no and goes on. The
//! seat asks for brave mode off as the session opens
//! (`session/set_config_option`, `brave_mode` `off`) and sends no message
//! before Junie confirmed it; a session that does not confirm it, or that
//! says later that it changed, takes the mind off the table, as does any
//! tool call that shows a tool ran (completed, failed without being
//! refused, or still open at the end of its turn). The seat offers Junie
//! no file system and no terminal of its own, and no MCP server. Measured
//! through this dialect on 31.10.2026 (26.10.27, `gemini-3.7-flash`): a
//! prompt that told the model to `touch` a file was asked of the seat,
//! refused, the file never made, and the next turn answered as usual.
//!
//! Locked down by flags as well (`docs/llm-seat.md` §"A CLI as the
//! model"): no user or project configuration
//! (`--config-default-locations=false`, and so none of its hooks), no MCP
//! server, skill, command, custom agent or custom model from their default
//! places, no extensions (a directory of the session's own, empty), no
//! guidelines (`--guidelines-filename` names an empty file of the
//! session's own; the working directory is empty besides), no update
//! check, no statistics, its caches in the session's directory, and ours
//! added to its system prompt (`--system-prompt`, which Junie adds to its
//! own: it cannot be replaced; in ACP mode too, as its log shows). The
//! model and the effort are its flags' (the session then names the model
//! by an id of its own, `v1:12:jetbrains-ai:gemini-3.7-flash`), and set
//! again through the session only where Junie offers the very value. Brave
//! mode, which approves actions, is never asked for (`--brave`), and no
//! key is ever passed (`--auth`, `--*-api-key`): Junie plays on the login
//! its owner signed in with.
//!
//! While it works Junie may start `systemd-inhibit` (its `keepAwakeMode`
//! setting, `WhileTaskRunning`), whose polkit agent reads the controlling
//! terminal: the seat starts it without one ([`super::launch::detach`]).
//!
//! A turn's reply is its `agent_message_chunk` text (Junie may add prose
//! around the answer's object), its usage the prompt result's, one turn's
//! own. Junie keeps each session under its home (`$JUNIE_HOME`, else
//! `~/.junie`, `sessions/<id>/`) with its settings and its fallback
//! credentials; the seat removes exactly its own directory when the
//! conversation is over ([`Dialect::session_files`]), and the line its
//! index (`sessions/index.jsonl`) holds for it stays.

use super::acp::{self, Agent, Want};
use super::dialect::{
    Dialect, Event, Started, Wire, conversation_id, entries, sounds_limited, tool_home,
};
use crate::llm::Settings;
use baylee_client_core::llmseat::CliTool;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The guidelines file Junie is pointed at: empty.
const GUIDELINES: &str = "guidelines.md";

/// The session option that has Junie approve its own actions, and the
/// value that has it ask the seat instead.
const BRAVE_MODE: (&str, &str) = ("brave_mode", "off");

/// Junie, as the protocol names it and reads its failures.
const AGENT: Agent = Agent {
    name: "junie",
    limited,
    signed_out,
};

/// The Junie CLI of `JetBrains`.
pub(crate) struct Junie;

impl Dialect for Junie {
    fn tool(&self) -> CliTool {
        CliTool::Junie
    }

    /// Its `systemd-inhibit` has polkit ask through `pkttyagent`, which
    /// reads the controlling terminal ([`super::launch::detach`]).
    fn detaches(&self) -> bool {
        true
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
            "--acp=true",
            "--skip-update-check",
            "--share-anonymous-statistics=false",
            "--config-default-locations=false",
            "--mcp-default-locations=false",
            "--skill-default-locations=false",
            "--command-default-location=false",
            "--agent-default-location=false",
            "--model-default-locations=false",
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

    fn sessions_root(&self, env: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
        tool_home(env, "JUNIE_HOME", ".junie").map(|home| home.join("sessions"))
    }

    fn session_files(&self, root: &Path, id: &str) -> Vec<PathBuf> {
        let Some(id) = conversation_id(id) else {
            return Vec::new();
        };
        entries(root)
            .into_iter()
            .filter(|(name, kind, _)| kind.is_dir() && *name == id)
            .map(|(_, _, path)| path)
            .collect()
    }

    fn opening(
        &self,
        settings: &Settings,
        model: Option<&str>,
        work: &Path,
        wire: &mut Wire,
    ) -> Vec<String> {
        let (brave, off) = BRAVE_MODE;
        let mut wants = vec![Want {
            id: brave,
            value: off.into(),
            must: true,
        }];
        if let Some(model) = model {
            // The model the seat declares is the one that plays: Junie's
            // ACP session ignores `--model` for a model it does not offer
            // (Junie Lite, measured 31.10.2026) and plays its default.
            wants.push(Want {
                id: "model",
                value: model.into(),
                must: true,
            });
        }
        if let Some(effort) = &settings.effort {
            wants.push(Want {
                id: "effort",
                value: effort.clone(),
                must: false,
            });
        }
        acp::open(wire, work, wants)
    }

    fn stdin_line(&self, text: &str) -> String {
        // The prompt's content; `message` sends it in the session's own
        // `session/prompt`.
        acp::content(text).to_string()
    }

    fn message(&self, text: &str, wire: &mut Wire) -> Vec<String> {
        acp::message(wire, text)
    }

    fn usage_is_cumulative(&self) -> bool {
        // A prompt result counts its own turn (measured 31.10.2026: a second
        // turn of one session read 18,645 input tokens after a first of
        // 17,723, while the running cost Junie reports doubled).
        false
    }

    fn read_event(&self, line: &str, wire: &mut Wire) -> Event {
        acp::read_event(line, wire, &AGENT)
    }

    fn lockdown_fault(&self, _started: &Started) -> Option<String> {
        // Its start names nothing: brave mode off is confirmed before any
        // message goes out, and every tool call after it is checked.
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

/// Whether Junie's words say a limit: a rate limit, a spent quota or
/// balance ("Insufficient account balance. All tokens in your account have
/// been spent."), its provider limited ("temporarily unavailable due to
/// rate limiting").
fn limited(said: &str) -> bool {
    sounds_limited(said) || said.to_lowercase().contains("balance")
}

/// Whether Junie's words say it is not signed in.
fn signed_out(said: &str) -> bool {
    let lower = said.to_lowercase();
    [
        "authentication",
        "authorization",
        "auth_required",
        "not signed in",
        "not logged in",
    ]
    .iter()
    .any(|sign| lower.contains(sign))
}
