//! What a CLI mind needs to know of one agent CLI: how its process is
//! started, what of the parent's environment it reads, how a message goes
//! in and what comes back out.
//!
//! A dialect knows nothing of sessions, clocks or budgets
//! ([`super::CliMind`] keeps those), and runs nothing: it turns settings
//! into arguments and files, messages into lines, and lines into
//! [`Event`]s. What one process's conversation keeps between lines (the
//! text of a reply so far, a failure an earlier line named, lines its
//! protocol answers with) is its [`Wire`], which the mind keeps beside the
//! process.
//!
//! Most dialects hold one conversation in one process for as long as it
//! runs and take each message as a line on its stdin: the process is the
//! session the mind keeps across turns, and so what keeps the
//! conversation's prefix in the provider's cache. A tool that answers one
//! message a process ([`Dialect::one_shot`]) gets every question as a new
//! conversation, the prefix and the seat's notes again, unless it resumes
//! one by its id ([`Dialect::resumes`]) from a store the seat owns: then
//! each process goes on with the conversation the last one left there.
//!
//! Which tools this build speaks, and how each is locked down, is
//! `docs/llm-seat.md` §"A CLI as the model".

use crate::llm::{Settings, Usage};
use baylee_client_core::llmseat::CliTool;
use serde_json::Value;
use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

/// One agent CLI's way of talking.
pub(crate) trait Dialect: Send + Sync {
    /// The tool it is.
    fn tool(&self) -> CliTool;

    /// Files the tool is pointed at, by name and contents: written, for
    /// this user alone, into the session's own `support` directory (never
    /// its working directory, which stays empty) before the process
    /// starts. A schema or instructions a flag takes only as a path.
    fn files(&self, _settings: &Settings, _system: &str) -> Vec<(&'static str, String)> {
        Vec::new()
    }

    /// The arguments of a session's process, after the program: `model` is
    /// the tool's own model id, where the profile names one, `system` the
    /// instructions, which replace the tool's own where it can, and
    /// `support` the directory [`Self::files`] are written to.
    fn args(
        &self,
        settings: &Settings,
        model: Option<&str>,
        system: &str,
        support: &Path,
    ) -> Vec<OsString>;

    /// The parent's variables the tool reads beyond the common ones
    /// ([`super::COMMON`]), passed where the parent has them: where its
    /// login lives. Never a key: [`super::forbidden`] refuses one by name
    /// whatever a dialect says here.
    fn passed_env(&self) -> &'static [&'static str];

    /// Variables the tool is given with fixed values, whatever the parent
    /// has: one that keeps a game's process from updating the tool, say.
    fn fixed_env(&self) -> &'static [(&'static str, &'static str)];

    /// Variables that name the session's own files in `support`, checked
    /// as every other variable is.
    fn session_env(&self, _support: &Path) -> Vec<(&'static str, OsString)> {
        Vec::new()
    }

    /// Whether a process takes one message and ends: its stdin is closed
    /// after that message, and each question starts a process of its own
    /// with the prefix and the seat's notes, as a conversation begun again
    /// is (not counted as a loss).
    fn one_shot(&self) -> bool {
        false
    }

    /// Whether a one-shot tool goes on with a conversation by its id
    /// ([`Wire::conversation`]), kept in a store of the seat's own: then a
    /// question within the conversation's limits is only what is new, sent
    /// to a process started with [`Self::resume_args`]. Such a tool must
    /// count each reply's own usage ([`Self::usage_is_cumulative`] false):
    /// a running count would start again with each process, though the
    /// conversation does not.
    fn resumes(&self) -> bool {
        false
    }

    /// Variables that put the tool's conversations into `store`, the
    /// seat's own directory ([`Self::resumes`]), and nowhere else; checked
    /// as every other variable is.
    fn store_env(&self, _store: &Path) -> Vec<(&'static str, OsString)> {
        Vec::new()
    }

    /// The arguments, after [`Self::args`], that go on with the
    /// conversation `id` ([`Self::resumes`]): an explicit id, never a
    /// tool's "the most recent one".
    fn resume_args(&self, _id: &str) -> Vec<OsString> {
        Vec::new()
    }

    /// The lines written as the process starts, before any message: a
    /// protocol's greeting and the conversation it opens.
    fn opening(
        &self,
        _settings: &Settings,
        _model: Option<&str>,
        _system: &str,
        _wire: &mut Wire,
    ) -> Vec<String> {
        Vec::new()
    }

    /// The line that sends `text` as the conversation's next message.
    fn stdin_line(&self, text: &str) -> String;

    /// The lines that send `text` as the conversation's next message: by
    /// default its [`Self::stdin_line`].
    fn message(&self, text: &str, _wire: &mut Wire) -> Vec<String> {
        vec![self.stdin_line(text)]
    }

    /// Whether a reply's usage is the process's running count since it
    /// started rather than the reply's own: then each call is booked as the
    /// difference to the reading before it from the same process, and a new
    /// process counts from nothing again. Summing running counts would book
    /// a long conversation many times over.
    fn usage_is_cumulative(&self) -> bool;

    /// Reads one line of the process's output. `wire` carries what an
    /// earlier line of the same reply said went wrong to the line that
    /// ends it, and takes the lines to write back ([`Wire::out`]).
    fn read_event(&self, line: &str, wire: &mut Wire) -> Event;

    /// What the model may use, as the process reported it at its start:
    /// why the seat does not play through it, or `None` when it named
    /// everything the lockdown asks it to and has nothing beyond the
    /// answer itself. A list it did not name is a fault where the tool
    /// names it at all: only a list it named shows the lockdown held.
    fn lockdown_fault(&self, started: &Started) -> Option<String>;

    /// The arguments of the login check, which calls no model: whether the
    /// tool is signed in. `None` for a tool that has none.
    fn probe_args(&self) -> Option<Vec<OsString>>;

    /// Whether the login check's output, from a process that exited
    /// cleanly, says the tool is signed in: what it wrote to stdout, and to
    /// stderr.
    fn probe_ok(&self, stdout: &[u8], stderr: &[u8]) -> bool;
}

/// What one process's conversation keeps between the lines it reads and
/// the messages it is sent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Wire {
    /// What an earlier line of the reply said went wrong, for the line
    /// that ends it.
    pub(crate) trouble: Option<String>,
    /// The conversation's id, where the tool names one: also whether its
    /// start was read.
    pub(crate) session: Option<String>,
    /// The id a tool that resumes ([`Dialect::resumes`]) named its
    /// conversation by, as [`conversation_id`] takes it.
    pub(crate) conversation: Option<String>,
    /// Lines to write to the process now: an answer to a request of its
    /// own, say.
    pub(crate) out: Vec<String>,
    /// The text of the reply so far, for a protocol that ends a reply on a
    /// line of its own.
    pub(crate) said: Option<String>,
    /// Whether the line just read is to be read once more: a tool with no
    /// start of its own starts on its first line, which says something of
    /// its own besides.
    pub(crate) again: bool,
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
    /// says; `None` where it named no source as text.
    pub(crate) key_source: Option<String>,
}

/// One line of output, read.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Event {
    /// The process started a conversation.
    Started(Started),
    /// The reply to the oldest message still unanswered.
    Reply(Outcome),
    /// The model used something the lockdown takes away (a tool, a
    /// command, a file): why. The mind is taken off the table, as for a
    /// start that offers it.
    Breach(String),
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

/// The names a start's list gives, each entry a name or an object with
/// one; an entry that is neither is kept as `(unnamed)`, so it is never
/// dropped unseen. `None` where `key` is not a list.
pub(crate) fn names(start: &Value, key: &str) -> Option<Vec<String>> {
    let items = start.get(key)?.as_array()?;
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
}

/// The start of a tool whose start vouches for nothing (it names no tools):
/// its first line of JSON with a `type`, whatever it says, which is then
/// read again for itself ([`Wire::again`]). So a failure that is its
/// first word (signed out, a configuration it refuses) is a reply after a
/// start, unavailable, and never taken for a reply before one, which
/// would take the mind off the table for good.
pub(crate) fn first_line_starts(value: &Value, wire: &mut Wire) -> Option<Event> {
    if wire.session.is_some() || !value.get("type").is_some_and(Value::is_string) {
        return None;
    }
    wire.session = Some(String::new());
    wire.again = true;
    Some(Event::Started(Started::default()))
}

/// `id` as a conversation's id to hand back to its tool in an argument:
/// letters, digits, `_` and `-`, not first, at most 128 of them; anything
/// else is no id (and so never resumed), so no line can make an argument
/// of its own.
pub(crate) fn conversation_id(id: &str) -> Option<String> {
    let fits = (1..=128).contains(&id.len())
        && !id.starts_with('-')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    fits.then(|| id.to_string())
}

/// Up to eight names, for a sentence.
pub(crate) fn listed(names: &[String]) -> String {
    let mut shown = names.iter().take(8).cloned().collect::<Vec<_>>().join(", ");
    if names.len() > 8 {
        shown.push_str(", …");
    }
    shown
}

/// Whether a limit's words say it is one: a rate limit, a usage limit, a
/// spent quota, HTTP 429, a provider's `RESOURCE_EXHAUSTED`.
pub(crate) fn sounds_limited(said: &str) -> bool {
    let lower = said.to_lowercase();
    [
        "rate limit",
        "rate_limit",
        "usage limit",
        "quota",
        "429",
        "resource_exhausted",
        "too many requests",
    ]
    .iter()
    .any(|sign| lower.contains(sign))
}

/// The first 200 characters of `said`.
pub(crate) fn clipped(said: &str) -> String {
    said.chars().take(200).collect()
}
