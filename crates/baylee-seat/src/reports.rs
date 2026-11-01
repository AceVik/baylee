//! What a language-model seat reports of the rules or the engine
//! (`docs/llm-protocol.md` §"Reports and lookups", `docs/llm-seat.md`
//! §"Reports and lookups").
//!
//! Any answer may carry a `report` beside its decision ([`Report`]): the
//! kind of error, how much it changes the game, how sure the model is, the
//! cards it names with the words of their text it quotes, the rule numbers
//! it cites, what happened and what should have. The seat adds what only
//! it may say ([`Checked`], [`Moment`]): the question it was answering, as
//! the seat showed it; whether the quotes are words of the card text the
//! seat itself told the model; whether the rule numbers are spelt as rule
//! numbers; `unverified` where there is neither; and the key a report is
//! filed once a game under.
//!
//! The seat files each one it reads ([`Book::file`]): one JSON line in
//! `reports.jsonl` at once, in its reports directory
//! (`llm::Settings::reports`: the transcripts' or a default one, so a
//! report the gateway never takes is kept), and kept for the game. When
//! the game is over the bridge sends each as a bug report of its own to
//! its gateway's `POST /reports` (`docs/feedback.md`) under the seat's own
//! session ([`submissions`]),
//! unless the bridge was told not to; a chair taken on its host's ticket,
//! which has no session and may send one a game, sends them all in one on
//! its seat token (`POST /lobby/games/{id}/chair/report`, [`bundle`]), and
//! the gateway files it under the host's pseudonym.
//!
//! A report is a model's words: untrusted, like a player's. It is marked
//! `client.source` (`llm-seat`, or `llm-trainer` for a model that watches
//! the trained AI play, [`Source`]) and its text says so, it is never read
//! as an instruction, it is never copied into code, and it goes nowhere
//! else. Triage checks it against the Comprehensive Rules and the Oracle;
//! the seat's checks only sort. Each dedupe key is filed once a game, at
//! most [`MOST`] of them, every field held to its bound.
//!
//! The bug report carries the seat's own game log beside it ([`game_log`]):
//! what a person's client at the seat attaches as "your game log", written
//! by the client's own code from what the table told this seat and nothing
//! else, with nobody's name in it. Each report says after which of its
//! lines it was made, and carries the last lines before it and the hash of
//! the whole log (`record_tail`, `record_sha256`). The game's record is the
//! gateway's to attach, by `game_id`: the seat has no record of the game,
//! only its own log, and attaches none (`local_record`).

mod body;
mod checks;
mod read;

pub use body::{RECORD_TAIL_CHARS, RECORD_TAIL_LINES, bundle, count, submissions};
pub use checks::{
    ANSWER_CHARS, Checked, ID_CHARS, MOST_OPTIONS, Moment, OPTION_CHARS, Source, dedupe_key,
    rule_number,
};
pub use read::{
    Card, Confidence, Kind, MOST_CARDS, MOST_RULES, NAME_CHARS, QUOTE_CHARS, RULE_CHARS, Report,
    Severity,
};

use crate::memory::TableMemory;
use baylee_client_core::bugreport::refs::{self, Candidates, CardRef};
use baylee_client_core::bugreport::{MAX_CLIENT_BYTES, Placeholders, SeatLog, seat_log_with};
use baylee_core::ids::CardIndex;
use baylee_view::PlayerView;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::Path;

/// The file a seat's reports are written to, in its reports directory.
pub const FILE: &str = "reports.jsonl";

/// The most reports one seat files in a game.
pub const MOST: usize = 25;

/// The longest a report's free text (`observed`, `expected`) is kept, in
/// characters.
pub const FIELD_CHARS: usize = 2_000;

/// The most references a report keeps.
pub const MOST_REFERENCES: usize = 20;

/// The longest a reference is kept, in characters.
pub const REFERENCE_CHARS: usize = 300;

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

/// A report as the seat filed it: where it was made, by which model, and
/// what the seat checked of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filed {
    /// Who filed it.
    pub source: Source,
    /// The game.
    pub game_id: String,
    /// The seat, as the view counts it.
    pub seat: u8,
    /// The question whose answer carried it.
    pub question: u64,
    /// The model's id.
    pub model: String,
    /// The profile it played, by name.
    pub profile: Option<String>,
    /// What it said.
    pub report: Report,
    /// What the seat checked of it.
    pub checked: Checked,
    /// The question it was made at, as the seat showed it.
    pub moment: Moment,
    /// The lookups made for that question before the answer.
    pub lookups: u32,
    /// When, in seconds since 1970.
    pub unix: u64,
    /// The cards it names that the seat's own view showed it, for the
    /// references of the bug report ([`submissions`]); not in the file.
    pub seen: Vec<CardRef>,
    /// How many lines of its game log the seat had been told when the
    /// question was asked: the report came after the last of them. Where
    /// the bug report's log puts it ([`submissions`]); not in the file.
    pub log_lines: u32,
}

impl Filed {
    /// The report as a bug report carries it (`client.report`, or an item
    /// of a bundle's `client.reports`): the seat's checks flat in it
    /// (`kind`, `oracle_quote_matches`, `rules_format_ok`, `dedupe_key`)
    /// beside the model's `severity` and `confidence`, the cards, the rule
    /// numbers and the question as the seat showed it (`decision`, with
    /// `log_line` where the log rides along), each held to its bound; then
    /// the model's own words. An answer of the old shape adds `evidence`.
    #[must_use]
    pub fn value(&self, log_line: Option<usize>) -> Value {
        let report = &self.report;
        let checked = &self.checked;
        let cards: Vec<Value> = report
            .cards
            .iter()
            .map(|card| {
                let mut value = json!({"name": card.name});
                if let Some(quote) = &card.oracle_quote {
                    value["oracle_quote"] = json!(quote);
                }
                value
            })
            .collect();
        let mut value = json!({
            "kind": checked.kind.as_str(),
            "severity": report.severity.map(Severity::as_str),
            "confidence": report.confidence.map(Confidence::as_str),
            "dedupe_key": checked.dedupe_key,
            "cards": cards,
            "rules": report.rules,
            "decision": self.moment.value(self.source, log_line),
            "observed": report.observed,
            "expected": report.expected,
            "references": report.references,
            "question": self.question,
            "lookups": self.lookups,
            "unix": self.unix,
        });
        if let Some(matches) = checked.oracle_quote_matches {
            value["oracle_quote_matches"] = json!(matches);
        }
        if let Some(ok) = checked.rules_format_ok {
            value["rules_format_ok"] = json!(ok);
        }
        if let Some(evidence) = &report.evidence {
            value["evidence"] = json!(evidence);
        }
        value
    }

    /// Its line in [`FILE`]: [`Filed::value`], and who filed it where.
    /// Never a key, a session or a name: the model's words, the seat's
    /// checks, and where they were said.
    #[must_use]
    pub fn line(&self) -> Value {
        let mut line = self.value(None);
        line["source"] = json!(self.source.as_str());
        line["game_id"] = json!(self.game_id);
        line["seat"] = json!(self.seat);
        line["turn"] = json!(self.moment.turn);
        line["model"] = json!(self.model);
        line["profile"] = json!(self.profile);
        line
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
    /// Every dedupe key filed: each once a game.
    keys: BTreeSet<String>,
}

impl Book {
    /// Files `filed`, unless its dedupe key was filed before or the game
    /// has [`MOST`]: kept for the bridge, and written to [`FILE`] in `dir`
    /// where there is one. Whether it was filed.
    pub fn file(&mut self, filed: Filed, dir: Option<&Path>) -> bool {
        if self.keys.len() >= MOST || !self.keys.insert(filed.checked.dedupe_key.clone()) {
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

/// Appends `filed`'s line to [`FILE`] in `dir`, in one write: the seats of a
/// table may share the directory.
///
/// Readable by the account the bridge runs under alone (`docs/privacy.md`):
/// on unix the file is made `0600`, whatever the umask, and the
/// directories made for it `0700`; a file an older build made readable by
/// others is narrowed to `0600` where this account may, and still written
/// where it may not. A directory that was there already keeps its mode (it
/// may be the transcripts' or one the operator chose). Elsewhere the
/// system's defaults hold.
///
/// # Errors
/// When the directory or the file cannot be written.
pub fn append(dir: &Path, filed: &Filed) -> std::io::Result<()> {
    let mut line = filed.line().to_string();
    line.push('\n');
    private_file(dir)?.write_all(line.as_bytes())
}

/// [`FILE`] in `dir`, open to append, made as [`append`] says.
fn private_file(dir: &Path) -> std::io::Result<std::fs::File> {
    let mut folders = std::fs::DirBuilder::new();
    folders.recursive(true);
    let mut options = std::fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _};
        folders.mode(0o700);
        options.mode(0o600);
    }
    folders.create(dir)?;
    let file = options.open(dir.join(FILE))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if file.metadata()?.permissions().mode() & 0o077 != 0 {
            // Another account's file in a shared directory cannot be
            // narrowed; the line is still written, as before.
            let _ = file.set_permissions(std::fs::Permissions::from_mode(0o600));
        }
    }
    Ok(file)
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
