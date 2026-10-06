//! Codex CLI, `codex`, as a mind's CLI: `codex exec`, one process per
//! message, the message on its stdin (`-`, read to its end) and JSONL
//! events on its stdout (`--json`).
//!
//! `codex exec` answers one prompt and ends ([`Dialect::one_shot`]), and
//! goes on with a thread by its id: `codex exec <flags> resume <id> -`, the
//! id the `thread.started` line named (a UUID, so never looked up as a
//! thread's name; never `--last`), read back from its rollout file
//! ([`Dialect::resumes`]). Codex keeps its sessions under `CODEX_HOME`
//! beside its login (`auth.json`, or a keyring entry keyed by that path),
//! and no setting moves them alone, so they are kept where the user's are
//! (`$CODEX_HOME/sessions/YYYY/MM/DD/rollout-<time>-<id>.jsonl`), and the
//! seat removes exactly its own when the conversation is over
//! ([`Dialect::session_files`]). Every flag below is given again to the
//! process that resumes; our instructions file outranks the copy the
//! rollout keeps. `turn.completed` carries the thread's running total,
//! seeded from the rollout, so it runs on across the processes of one
//! conversation ([`Dialect::usage_spans_resumes`]). (`codex app-server`
//! holds a thread over stdio, but is marked experimental and its wire
//! spellings disagree between its docs and its source; it is not used.)
//!
//! Locked down by flags and `-c` overrides (`docs/llm-seat.md` §"A CLI as
//! the model"): our instructions replace Codex's own
//! (`model_instructions_file`), no `AGENTS.md` of a project
//! (`project_doc_max_bytes=0`), no user `config.toml` and so none of its MCP
//! servers, hooks, profiles or notify command (`--ignore-user-config`, and
//! `mcp_servers={}`, `notify=[]` for good measure), no exec-policy rules
//! (`--ignore-rules`), no shell, no exec, no image, web, plugin, app, skill,
//! hook, memory or sub-agent tools (`--disable <feature>`,
//! `web_search="disabled"`), a read-only sandbox, no prompt history
//! (`history.persistence="none"`; the thread's rollout is kept, which a
//! resume reads), no analytics, feedback,
//! telemetry or update check, and the answer's schema
//! (`--output-schema`). Nothing is approved for it: `exec` asks nobody,
//! and the read-only sandbox refuses what a tool would write.
//!
//! Codex names no tools at its start: its `thread.started` line is the
//! start, and the proof the lockdown held is read off every line after
//! it, so an item of any kind but the answer's own message, the model's
//! reasoning or a warning (a command run, a file changed, an MCP or web
//! call, a plan) takes the mind off the table ([`Event::Breach`]).
//! `$CODEX_HOME/AGENTS.md` is still read: Codex has no switch for it, and
//! the bridge warns where one is there ([`Dialect::home_warning`]).

use super::dialect::{
    Dialect, Event, Outcome, Started, Wire, clipped, conversation_id, entries, first_line_starts,
    sounds_limited, tool_home,
};
use crate::llm::{Settings, Usage, json_object, prompt};
use baylee_client_core::llmseat::CliTool;
use serde_json::Value;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The file our instructions are written to.
const INSTRUCTIONS: &str = "instructions.md";

/// The file the answer's schema is written to.
const SCHEMA: &str = "schema.json";

/// Codex's features that put a tool before the model or reach beyond the
/// answer, each turned off (`--disable`). Codex ignores a name it does not
/// know.
pub(crate) const DISABLED_FEATURES: [&str; 18] = [
    "shell_tool",
    "unified_exec",
    "view_image",
    "sleep_tool",
    "multi_agent",
    "apps",
    "plugins",
    "hooks",
    "skill_search",
    "skill_mcp_dependency_install",
    "image_generation",
    "goals",
    "browser_use",
    "computer_use",
    "in_app_browser",
    "tool_suggest",
    "memories",
    "web_search_request",
];

/// The item types a locked-down turn may hold: the answer, the model's
/// reasoning, and a warning Codex says about itself.
const ALLOWED_ITEMS: [&str; 3] = ["agent_message", "reasoning", "error"];

/// Codex CLI.
pub(crate) struct Codex;

/// A TOML string, for a `-c` value: JSON's escapes are TOML's.
fn toml_string(text: &str) -> String {
    Value::from(text).to_string()
}

impl Dialect for Codex {
    fn tool(&self) -> CliTool {
        CliTool::Codex
    }

    fn files(&self, _settings: &Settings, system: &str) -> Vec<(&'static str, String)> {
        vec![
            (INSTRUCTIONS, system.to_string()),
            (SCHEMA, prompt::answer_schema().to_string()),
        ]
    }

    fn args(
        &self,
        settings: &Settings,
        model: Option<&str>,
        _system: &str,
        support: &Path,
    ) -> Vec<OsString> {
        let mut args: Vec<OsString> = [
            "exec",
            "--json",
            "--color",
            "never",
            "--ignore-user-config",
            "--ignore-rules",
            "--skip-git-repo-check",
            "--sandbox",
            "read-only",
            "--output-schema",
        ]
        .into_iter()
        .map(OsString::from)
        .collect();
        args.push(support.join(SCHEMA).into());
        let instructions = support.join(INSTRUCTIONS);
        let mut config = vec![
            format!(
                "model_instructions_file={}",
                toml_string(&instructions.to_string_lossy())
            ),
            "project_doc_max_bytes=0".into(),
            "mcp_servers={}".into(),
            "notify=[]".into(),
            "web_search=\"disabled\"".into(),
            "tools.view_image=false".into(),
            "history.persistence=\"none\"".into(),
            "analytics.enabled=false".into(),
            "feedback.enabled=false".into(),
            "otel.exporter=\"none\"".into(),
            "check_for_update_on_startup=false".into(),
            "include_environment_context=false".into(),
            "include_permissions_instructions=false".into(),
            "include_apps_instructions=false".into(),
            "model_reasoning_summary=\"none\"".into(),
        ];
        if let Some(effort) = &settings.effort {
            config.push(format!("model_reasoning_effort={}", toml_string(effort)));
        }
        for value in config {
            args.push("-c".into());
            args.push(value.into());
        }
        for feature in DISABLED_FEATURES {
            args.push("--disable".into());
            args.push(feature.into());
        }
        if let Some(model) = model {
            args.push("--model".into());
            args.push(model.into());
        }
        // The prompt is stdin's, read to its end.
        args.push("-".into());
        args
    }

    fn passed_env(&self) -> &'static [&'static str] {
        &["CODEX_HOME"]
    }

    fn resumes(&self) -> bool {
        true
    }

    fn usage_spans_resumes(&self) -> bool {
        true
    }

    fn resume(&self, args: &mut Vec<OsString>, id: &str) {
        // `exec`'s own flags stay ahead of the subcommand, where `exec`
        // reads them (`--sandbox` and `--color` are not `resume`'s); the
        // prompt is still stdin's.
        if args.last().is_some_and(|last| last == "-") {
            args.pop();
        }
        args.extend(["resume".into(), id.into(), "-".into()]);
    }

    fn sessions_root(&self, env: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
        tool_home(env, "CODEX_HOME", ".codex").map(|home| home.join("sessions"))
    }

    fn session_files(&self, root: &Path, id: &str) -> Vec<PathBuf> {
        // `sessions/YYYY/MM/DD/rollout-YYYY-MM-DDTHH-MM-SS-<id>.jsonl`,
        // compressed (`.zst`) after a week.
        let Some(id) = thread_id(id) else {
            return Vec::new();
        };
        let mut files = Vec::new();
        let dirs = |dir: &Path, digits: usize| -> Vec<PathBuf> {
            entries(dir)
                .into_iter()
                .filter(|(name, kind, _)| {
                    kind.is_dir()
                        && name.len() == digits
                        && name.chars().all(|c| c.is_ascii_digit())
                })
                .map(|(_, _, path)| path)
                .collect()
        };
        for year in dirs(root, 4) {
            for month in dirs(&year, 2) {
                for day in dirs(&month, 2) {
                    files.extend(
                        entries(&day)
                            .into_iter()
                            .filter(|(name, kind, _)| kind.is_file() && rollout_of(name, &id))
                            .map(|(_, _, path)| path),
                    );
                }
            }
        }
        files
    }

    fn home_warning(&self, env: &dyn Fn(&str) -> Option<OsString>) -> Option<String> {
        let agents = tool_home(env, "CODEX_HOME", ".codex")?.join("AGENTS.md");
        std::fs::symlink_metadata(&agents).is_ok().then(|| {
            format!(
                "{} is there, and codex reads it into every seat's instructions: no flag turns                  it off",
                agents.display()
            )
        })
    }

    fn fixed_env(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }

    fn one_shot(&self) -> bool {
        true
    }

    fn stdin_line(&self, text: &str) -> String {
        // The whole of stdin is the prompt, as it stands.
        text.to_string()
    }

    fn usage_is_cumulative(&self) -> bool {
        // `turn.completed` carries the thread's running total (the exec
        // event processor's `usage_from_last_total`), seeded from the
        // rollout on a resume: booked as its differences across the
        // conversation.
        true
    }

    fn read_event(&self, line: &str, wire: &mut Wire) -> Event {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return Event::Other;
        };
        if wire.conversation.is_none()
            && value.get("type").and_then(Value::as_str) == Some("thread.started")
        {
            wire.conversation = value
                .get("thread_id")
                .and_then(Value::as_str)
                .and_then(thread_id);
        }
        if let Some(start) = first_line_starts(&value, wire) {
            return start;
        }
        let text = |key: &str| value.get(key).and_then(Value::as_str);
        match text("type") {
            Some("item.started" | "item.updated" | "item.completed") => {
                let item = value.get("item").unwrap_or(&Value::Null);
                let kind = item.get("type").and_then(Value::as_str).unwrap_or("(none)");
                if !ALLOWED_ITEMS.contains(&kind) {
                    return Event::Breach(format!(
                        "the codex process let the model use {kind}: the seat does not play \
                         through it"
                    ));
                }
                if text("type") == Some("item.completed") && kind == "agent_message" {
                    wire.said = item.get("text").and_then(Value::as_str).map(str::to_string);
                }
                Event::Other
            }
            // Not the end: Codex says so of a retry it makes itself.
            Some("error") => {
                if let Some(message) = text("message") {
                    wire.trouble = Some(message.to_string());
                }
                Event::Other
            }
            Some("turn.completed") => {
                let said = wire.said.take().unwrap_or_default();
                wire.trouble = None;
                Event::Reply(Outcome::Answer {
                    value: json_object(&said),
                    text: said,
                    usage: value.get("usage").map(usage),
                })
            }
            Some("turn.failed") => {
                wire.said = None;
                let why = value
                    .get("error")
                    .and_then(|error| error.get("message"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .or_else(|| wire.trouble.take())
                    .unwrap_or_else(|| "the turn failed".into());
                wire.trouble = None;
                Event::Reply(failure(&why))
            }
            _ => Event::Other,
        }
    }

    fn lockdown_fault(&self, _started: &Started) -> Option<String> {
        // Its start names nothing; every later line is checked instead.
        None
    }

    fn probe_args(&self) -> Option<Vec<OsString>> {
        Some(["login", "status"].map(OsString::from).into())
    }

    fn probe_ok(&self, stdout: &[u8], stderr: &[u8]) -> bool {
        // `codex login status` says it on stderr. A key is no
        // subscription: only a sign-in plays.
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(stdout),
            String::from_utf8_lossy(stderr)
        );
        said.lines()
            .any(|line| line.starts_with("Logged in using") && !line.contains("API key"))
    }
}

/// A failed turn: a usage or rate limit by its words, else a failure.
fn failure(why: &str) -> Outcome {
    if sounds_limited(why) {
        Outcome::RateLimited {
            why: clipped(why),
            lifts_in: None,
        }
    } else {
        Outcome::Failed(clipped(why))
    }
}

/// The tokens `turn.completed` counted: `OpenAI`'s `input_tokens` holds the
/// cached ones, which are read here as cache reads.
fn usage(usage: &Value) -> Usage {
    let n = |key: &str| usage.get(key).and_then(Value::as_u64).unwrap_or(0);
    let cached = n("cached_input_tokens");
    Usage {
        input: n("input_tokens").saturating_sub(cached),
        output: n("output_tokens"),
        cache_write: n("cache_write_input_tokens"),
        cache_read: cached,
        ..Usage::default()
    }
}

/// `id` as a thread's id: a UUID, which `resume` never looks up as a
/// name, in its hyphenated spelling.
fn thread_id(id: &str) -> Option<String> {
    let uuid = uuid::Uuid::parse_str(id).ok()?;
    let spelled = uuid.hyphenated().to_string();
    (spelled == id.to_ascii_lowercase())
        .then_some(spelled)
        .and_then(|id| conversation_id(&id))
}

/// Whether `name` is the rollout file of thread `id`:
/// `rollout-YYYY-MM-DDTHH-MM-SS-<id>.jsonl`, or `.jsonl.zst`.
fn rollout_of(name: &str, id: &str) -> bool {
    let Some(rest) = name.strip_prefix("rollout-") else {
        return false;
    };
    let Some(rest) = rest
        .strip_suffix(".jsonl")
        .or_else(|| rest.strip_suffix(".jsonl.zst"))
    else {
        return false;
    };
    let Some((time, tail)) = rest.split_at_checked(19) else {
        return false;
    };
    time.chars()
        .all(|c| c.is_ascii_digit() || c == '-' || c == 'T')
        && tail.strip_prefix('-') == Some(id)
}
