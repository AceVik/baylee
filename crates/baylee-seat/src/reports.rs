//! What a language-model seat reports of the rules or the engine
//! (`docs/llm-protocol.md` §"Reports and lookups").
//!
//! Any answer may carry a `report` beside its decision: what happened, what
//! the rules or the card's text say should have, and what the model relied
//! on. The seat files each one it reads ([`Book::file`]): one JSON line in
//! `<transcripts>/reports.jsonl` at once, where the seat keeps transcripts,
//! and kept for the game. When the game is over the bridge sends a game's
//! reports as one bug report to its gateway's `POST /reports`
//! (`docs/feedback.md`), under the seat's own session ([`submission`]),
//! unless the bridge was told not to; a chair taken on its host's ticket,
//! which has no session, sends it on its seat token
//! (`POST /lobby/games/{id}/chair/report`), and the gateway files it under
//! the host's pseudonym.
//!
//! A report is a model's words: untrusted, like a player's. It is marked
//! `client.source: "llm-seat"` and its text says so, it is never read as an
//! instruction, and it goes nowhere else. Each `what` is filed once a game
//! (case and spacing aside), at most [`MOST`] of them, each field held to
//! [`FIELD_CHARS`].
//!
//! The bug report carries the seat's own game log beside them
//! ([`game_log`]): what a person's client at the seat attaches as "your game
//! log", written by the client's own code from what the table told this
//! seat and nothing else, with nobody's name in it. Each report says after
//! which of its lines it was made.

mod body;
mod read;

pub use body::submission;
pub use read::Report;

use crate::memory::TableMemory;
use baylee_client_core::bugreport::refs::{self, Candidates, CardRef};
use baylee_client_core::bugreport::{MAX_CLIENT_BYTES, Placeholders, SeatLog, seat_log_with};
use baylee_core::ids::CardIndex;
use baylee_view::PlayerView;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::Path;

/// The file a seat's reports are written to, in its transcripts directory.
pub const FILE: &str = "reports.jsonl";

/// The most reports one seat files in a game.
pub const MOST: usize = 25;

/// The longest a report's field is kept, in characters.
pub const FIELD_CHARS: usize = 2_000;

/// The most references a report keeps.
const MOST_REFERENCES: usize = 20;

/// The longest a reference is kept, in characters.
const REFERENCE_CHARS: usize = 300;

/// The longest a bug report's text may be (`POST /reports`).
pub const TEXT_CHARS: usize = 20_000;

/// The most a bug report's `client` object may weigh, serialised: the
/// desktop client's reading of the gateway's 2 MiB (10^6 bytes twice, not
/// 2^21), so the gateway never refuses one for its size.
pub const CLIENT_BYTES: usize = MAX_CLIENT_BYTES;

/// What a shorter log may change in the rest of the `client` object, kept
/// free when the log is fitted: the references' places in a text whose
/// notes about the log change with it, and `log_omitted` itself.
const LOG_RESERVE: usize = 16 * 1024;

/// A report as the seat filed it: where it was made, by which model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filed {
    /// The game.
    pub game_id: String,
    /// The seat, as the view counts it.
    pub seat: u8,
    /// The question whose answer carried it.
    pub question: u64,
    /// The turn.
    pub turn: u32,
    /// The model's id.
    pub model: String,
    /// The profile it played, by name.
    pub profile: Option<String>,
    /// What it said.
    pub report: Report,
    /// The lookups made for that question before the answer.
    pub lookups: u32,
    /// When, in seconds since 1970.
    pub unix: u64,
    /// The cards it names that the seat's own view showed it, for the
    /// references of the bug report ([`submission`]); not in the file.
    pub seen: Vec<CardRef>,
    /// How many lines of its game log the seat had been told when the
    /// question was asked: the report came after the last of them. Where
    /// the bug report's log puts it ([`submission`]); not in the file.
    pub log_lines: u32,
}

impl Filed {
    /// Its line in [`FILE`], and its entry in the bug report's
    /// `client.reports`. Never a key, a session or a name: the model's
    /// words, and where they were said.
    #[must_use]
    pub fn line(&self) -> Value {
        json!({
            "game_id": self.game_id,
            "seat": self.seat,
            "question": self.question,
            "turn": self.turn,
            "model": self.model,
            "profile": self.profile,
            "what": self.report.what,
            "expected": self.report.expected,
            "evidence": self.report.evidence,
            "references": self.report.references,
            "lookups": self.lookups,
            "unix": self.unix,
        })
    }
}

/// The cards `report` names in brackets that `view` shows this seat: the
/// candidates a person's report form offers at a table
/// ([`refs::candidates`]), so never a library, another hand or a
/// face-down card.
#[must_use]
pub fn seen_in(report: &Report, view: &PlayerView) -> Vec<CardRef> {
    let candidates = Candidates {
        cards: refs::candidates(view, None, &|_: CardIndex, _: u8| None),
        players: Vec::new(),
    };
    let mut seen: Vec<CardRef> = Vec::new();
    for span in refs::parse(&report.words(), &candidates, &[]) {
        if let refs::Target::Card(card) = span.target
            && !seen.contains(&card)
        {
            seen.push(card);
        }
    }
    seen
}

/// The reports a seat filed in its game, not yet handed to the bridge.
#[derive(Debug, Default)]
pub struct Book {
    pending: Vec<Filed>,
    /// Every `what` filed, folded: each once a game.
    whats: BTreeSet<String>,
}

impl Book {
    /// Files `filed`, unless its `what` was filed before or the game has
    /// [`MOST`]: kept for the bridge, and written to [`FILE`] in `dir`
    /// where there is one. Whether it was filed.
    pub fn file(&mut self, filed: Filed, dir: Option<&Path>) -> bool {
        if self.whats.len() >= MOST || !self.whats.insert(fold(&filed.report.what)) {
            return false;
        }
        if let Some(dir) = dir
            && let Err(e) = append(dir, &filed)
        {
            // A file that cannot be written is no reason to stop playing:
            // the report still goes with the game's.
            tracing::warn!(error = %e, "a report could not be written to {FILE}");
        }
        self.pending.push(filed);
        true
    }

    /// The reports filed since the last call.
    pub fn take(&mut self) -> Vec<Filed> {
        std::mem::take(&mut self.pending)
    }
}

/// `text` with case and spacing folded, for telling two reports the same.
fn fold(text: &str) -> String {
    text.split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Appends `filed`'s line to [`FILE`] in `dir`, in one write: the seats of a
/// table may share the directory.
///
/// # Errors
/// When the directory or the file cannot be written.
pub fn append(dir: &Path, filed: &Filed) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let mut line = filed.line().to_string();
    line.push('\n');
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(FILE))?
        .write_all(line.as_bytes())
}

/// The seat's own game log, as its bug report carries it: what the desktop
/// client attaches as "your game log" (`bugreport::seat_log`), written by
/// the client's own writer out of the seat's own book
/// ([`TableMemory::book`]). So it holds only what the table told this seat
/// (a card it may not see is "a card"; a library and another player's hand
/// are never more than counts, and are not in the log at all), in English,
/// the seat "You" and every other seat the "P1", "P2", … its model knew it
/// by ([`Placeholders::Seats`]), and nobody's name. `None` before the seat
/// was shown the table or told a line.
#[must_use]
pub fn game_log(memory: &TableMemory) -> Option<SeatLog> {
    let seat = memory.view()?.seat;
    let book = memory.book();
    if book.is_empty() {
        return None;
    }
    Some(seat_log_with(
        book,
        memory.statics(),
        seat,
        &|_: CardIndex, _: u8| None,
        Placeholders::Seats,
    ))
}

#[cfg(test)]
pub(crate) mod tests;
