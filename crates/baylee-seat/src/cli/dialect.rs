//! What a CLI mind needs to know of one agent CLI: how its process is
//! started, what of the parent's environment it reads, how a message goes
//! in and what comes back out.
//!
//! A dialect knows nothing of sessions, clocks or budgets
//! ([`super::CliMind`] keeps those), and runs nothing: it turns settings
//! into arguments and lines into [`Event`]s. This build speaks Claude Code
//! ([`super::claude`]), whose process holds one conversation and takes
//! each message as a line on its stdin.

use crate::llm::{Settings, Usage};
use baylee_client_core::llmseat::CliTool;
use serde_json::Value;
use std::ffi::OsString;
use std::time::Duration;

/// One agent CLI's way of talking.
pub(crate) trait Dialect: Send + Sync {
    /// The tool it is.
    fn tool(&self) -> CliTool;

    /// The arguments of a session's process, after the program: `model` is
    /// the tool's own model id, where the profile names one, and `system`
    /// the instructions, which replace the tool's own.
    fn args(&self, settings: &Settings, model: Option<&str>, system: &str) -> Vec<OsString>;

    /// The parent's variables the tool reads beyond the common ones
    /// ([`super::COMMON`]), passed where the parent has them: where its
    /// login lives. Never a key: [`super::forbidden`] refuses one by name
    /// whatever a dialect says here.
    fn passed_env(&self) -> &'static [&'static str];

    /// Variables the tool is given with fixed values, whatever the parent
    /// has: one that keeps a game's process from updating the tool, say.
    fn fixed_env(&self) -> &'static [(&'static str, &'static str)];

    /// The line that sends `text` as the conversation's next message.
    fn stdin_line(&self, text: &str) -> String;

    /// Reads one line of the process's output. `trouble` carries what an
    /// earlier line of the same reply said went wrong to the line that
    /// ends it.
    fn read_event(&self, line: &str, trouble: &mut Option<String>) -> Event;

    /// What the model may use, as the process reported it at its start:
    /// why the seat does not play through it, or `None` when it named
    /// everything the lockdown asks it to and has nothing beyond the
    /// answer itself. A list it did not name is a fault: only a list it
    /// named shows the lockdown held.
    fn lockdown_fault(&self, started: &Started) -> Option<String>;

    /// The arguments of the login check, which calls no model: whether the
    /// tool is signed in. `None` for a tool that has none.
    fn probe_args(&self) -> Option<Vec<OsString>>;

    /// Whether the login check's output, from a process that exited
    /// cleanly, says the tool is signed in.
    fn probe_ok(&self, stdout: &[u8]) -> bool;
}

/// What the process said it started with: each list `None` where the
/// line did not name it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Started {
    /// The tools the model is offered.
    pub(crate) tools: Option<Vec<String>>,
    /// The MCP servers it is connected to.
    pub(crate) mcp_servers: Option<Vec<String>>,
    /// The slash commands and skills the conversation could run.
    pub(crate) slash_commands: Option<Vec<String>>,
    /// Where the credential it calls the model with comes from, as it
    /// says; `None` where it named no source as text, which is a fault.
    pub(crate) key_source: Option<String>,
}

/// One line of output, read.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Event {
    /// The process started a conversation.
    Started(Started),
    /// The reply to the oldest message still unanswered.
    Reply(Outcome),
    /// Anything else: the model's own messages, progress.
    Other,
}

/// How a message was answered.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Outcome {
    /// An answer: the object the tool held to the schema, where it gave
    /// one, else whatever object the text holds; the text; the tokens the
    /// tool counted, where it said.
    Answer {
        value: Option<Value>,
        text: String,
        usage: Option<Usage>,
    },
    /// A rate limit or a spent quota, unbilled: why, and how long until it
    /// lifts where the tool said.
    RateLimited {
        why: String,
        lifts_in: Option<Duration>,
    },
    /// Anything else that went wrong, billed as far as anybody knows.
    Failed(String),
}
