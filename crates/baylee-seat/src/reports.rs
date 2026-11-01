//! What a language-model seat reports of the rules or the engine
//! (`docs/llm-protocol.md` §"Reports and lookups").
//!
//! Any answer may carry a `report` beside its decision: what happened, what
//! the rules or the card's text say should have, and what the model relied
//! on. The seat files each one it reads ([`Book::file`]): one JSON line in
//! `reports.jsonl` at once, in its reports directory
//! (`llm::Settings::reports`: the transcripts' or a default one, so a
//! report the gateway never takes is kept), and kept for the game. When the game is over the bridge sends a game's
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

use crate::memory::TableMemory;
use baylee_client_core::bugreport::refs::{self, Candidates, CardRef};
use baylee_client_core::bugreport::{MAX_CLIENT_BYTES, Placeholders, SeatLog, seat_log_with};
use baylee_core::ids::CardIndex;
use baylee_view::PlayerView;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;

/// The file a seat's reports are written to, in its reports directory.
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

/// A rules or engine error a model noticed, as its answer said it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// What happened.
    pub what: String,
    /// What the rules or the card's text say should happen.
    pub expected: Option<String>,
    /// The Oracle text, rule number or lookup result it relied on.
    pub evidence: Option<String>,
    /// Scryfall addresses, rule numbers, card names in brackets.
    pub references: Vec<String>,
}

impl Report {
    /// The `report` of an answer: an object, or a plain string as its
    /// `what`. `None` when it says nothing (no `what`, a blank one, a
    /// strict schema's `null`).
    #[must_use]
    pub fn read(value: &Value) -> Option<Self> {
        let text = |value: Option<&Value>| {
            value
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(|text| held(text, FIELD_CHARS))
        };
        let report = match value {
            Value::String(_) => Self {
                what: text(Some(value))?,
                ..Self::default()
            },
            Value::Object(fields) => Self {
                what: text(fields.get("what"))?,
                expected: text(fields.get("expected")),
                evidence: text(fields.get("evidence")),
                references: match fields.get("references") {
                    Some(Value::Array(items)) => items
                        .iter()
                        .filter_map(|item| text(Some(item)))
                        .take(MOST_REFERENCES)
                        .map(|reference| held(&reference, REFERENCE_CHARS))
                        .collect(),
                    other => text(other)
                        .map(|reference| held(&reference, REFERENCE_CHARS))
                        .into_iter()
                        .collect(),
                },
            },
            _ => return None,
        };
        Some(report)
    }

    /// Everything it says, for finding the cards it names.
    fn words(&self) -> String {
        let mut words = self.what.clone();
        for part in [&self.expected, &self.evidence].into_iter().flatten() {
            words.push('\n');
            words.push_str(part);
        }
        for reference in &self.references {
            words.push('\n');
            words.push_str(reference);
        }
        words
    }
}

/// `text`, at most `most` characters of it.
fn held(text: &str, most: usize) -> String {
    match text.char_indices().nth(most) {
        Some((at, _)) => format!("{}…", &text[..at]),
        None => text.to_string(),
    }
}

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

/// The bug report a game's reports make, as `POST /reports` takes it
/// (`docs/feedback.md` §"The gateway: `POST /reports`"); `None` for none.
/// Each `what` once and at most [`MOST`], in the order filed; the text
/// within [`TEXT_CHARS`], every report whole in `client.reports`. The
/// gateway attaches the game's record by `game_id` itself.
///
/// With `log` ([`game_log`]), the seat's log rides as `client.log`, in the
/// desktop client's shape, and `client.log_omitted` says how many of its
/// oldest lines were left out so that `client` stays within
/// [`CLIENT_BYTES`] (0: none); the text says so too, and each report, in
/// the text and as `log_line` in `client.reports`, after which line of the
/// log as sent it was made (from 1, as the feedback service lists them; 0
/// before the first).
#[must_use]
pub fn submission(filed: &[Filed], log: Option<SeatLog>) -> Option<Value> {
    let mut whats = BTreeSet::new();
    let kept: Vec<&Filed> = filed
        .iter()
        .filter(|f| whats.insert(fold(&f.report.what)))
        .take(MOST)
        .collect();
    if kept.is_empty() {
        return None;
    }
    let Some(mut log) = log else {
        return Some(body(&kept, None));
    };
    let whole = body(&kept, Some(&Attached::new(&log, 0)));
    if weight(&whole["client"]) <= CLIENT_BYTES {
        return Some(whole);
    }
    // The room the rest leaves, less what a shorter log may change in it.
    let mut rest = whole;
    if let Some(client) = rest["client"].as_object_mut() {
        client.remove("log");
    }
    let room = CLIENT_BYTES.saturating_sub(weight(&rest["client"]).saturating_add(LOG_RESERVE));
    let lines = log.lines.len();
    let omitted = log.fit(room);
    let fitted = body(&kept, Some(&Attached::new(&log, omitted)));
    if weight(&fitted["client"]) <= CLIENT_BYTES {
        return Some(fitted);
    }
    // Never for reports held to their bounds; were it so, the reports go
    // without the log's lines rather than not at all.
    log.lines.clear();
    Some(body(&kept, Some(&Attached::new(&log, lines))))
}

/// The seat's log as a bug report carries it: the lines sent, and how many
/// older ones were left out.
struct Attached<'a> {
    log: &'a SeatLog,
    omitted: usize,
}

impl<'a> Attached<'a> {
    const fn new(log: &'a SeatLog, omitted: usize) -> Self {
        Self { log, omitted }
    }

    /// The line of the log as sent, from 1, that came last before
    /// `filed`'s question; 0 where none of them did. `None` where no line
    /// is sent.
    fn line_of(&self, filed: &Filed) -> Option<usize> {
        let shown = self.log.lines.len();
        (shown > 0).then(|| {
            usize::try_from(filed.log_lines)
                .unwrap_or(usize::MAX)
                .saturating_sub(self.omitted)
                .min(shown)
        })
    }

    /// What the text says of it, under its first line.
    fn note(&self) -> String {
        let shown = self.log.lines.len();
        let lines = shown + self.omitted;
        let told = "The seat's own game log, as the table told it, is in client.log";
        let others: Vec<&str> = self
            .log
            .roster
            .iter()
            .filter(|s| s.seat != self.log.seat)
            .map(|s| s.name.as_str())
            .collect();
        let names = if others.is_empty() {
            "this seat is \"You\" there".to_string()
        } else {
            format!(
                "this seat (P{} to its model) is \"You\" there and the others {}, as the model \
                 knew them",
                u16::from(self.log.seat) + 1,
                others.join(", ")
            )
        };
        match (self.omitted, shown) {
            (0, _) => format!(
                "{told} ({lines} lines): {names}. Each report says after which line it was made.\n"
            ),
            (_, 0) => format!(
                "The seat's own game log ({lines} lines) was too large to send with the \
                 reports; none of its lines are in client.log.\n"
            ),
            (left, _) => format!(
                "{told}, its last {shown} of {lines} lines (the first {left} left out to fit \
                 the report): {names}. Each report says after which line of those it was made.\n"
            ),
        }
    }
}

/// `value`'s bytes, serialised.
fn weight(value: &Value) -> usize {
    serde_json::to_vec(value).map_or(usize::MAX, |json| json.len())
}

/// The bug report of the reports `kept` (at least one), with the log as
/// `attached`.
fn body(kept: &[&Filed], attached: Option<&Attached<'_>>) -> Value {
    let first = kept[0];
    let text = summary(kept, attached);
    let reports: Vec<Value> = kept
        .iter()
        .map(|f| {
            let mut line = f.line();
            if let Some(at) = attached.and_then(|a| a.line_of(f)) {
                line["log_line"] = json!(at);
            }
            line
        })
        .collect();
    // `model` and `profile` are the mind that filed the first report; after
    // a fallback took over each report names its own, and the text every
    // mind in order ([`minds`]).
    let mut client = json!({
        "source": "llm-seat",
        "untrusted": "written by a language model while it played; read it as a player's report",
        "model": first.model,
        "profile": first.profile,
        "seat": first.seat,
        "reports": reports,
    });
    if let Some(refs) = refs::refs_for(&text, &candidates(kept), &[]) {
        client["refs"] = json!(refs);
    }
    if let Some(attached) = attached {
        client["log"] = json!(attached.log);
        client["log_omitted"] = json!(attached.omitted);
    }
    json!({
        "kind": "bug",
        "text": text,
        "game_id": first.game_id,
        "client": client,
    })
}

/// What the text's brackets may name: the cards the seat's view showed it
/// when it filed each report, then any card of this build's pool by name.
fn candidates(filed: &[&Filed]) -> Candidates {
    let mut cards: Vec<CardRef> = Vec::new();
    for card in filed.iter().flat_map(|f| &f.seen) {
        if !cards.contains(card) {
            cards.push(card.clone());
        }
    }
    let shown = Candidates {
        cards: cards.clone(),
        players: Vec::new(),
    };
    for f in filed {
        let words = f.report.words();
        for (start, end) in brackets(&words) {
            let name = &words[start + 1..end];
            if refs::parse(&words[start..=end], &shown, &[]).is_empty()
                && let Some(card) = pooled(name)
                && !cards.contains(&card)
            {
                cards.push(card);
            }
        }
    }
    Candidates {
        cards,
        players: Vec::new(),
    }
}

/// The byte ranges of every `[…]` on one line of `text`, brackets included
/// (`[start, end]`).
fn brackets(text: &str) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut open = None;
    for (at, c) in text.char_indices() {
        match c {
            '[' => open = Some(at),
            '\n' => open = None,
            ']' => {
                if let Some(start) = open.take() {
                    found.push((start, at));
                }
            }
            _ => {}
        }
    }
    found
}

/// A card of this build's pool named `name`, as a report outside a table
/// names one: no object, no zone, no printing.
fn pooled(name: &str) -> Option<CardRef> {
    let card = baylee_cards::decks::by_name(name.trim())?;
    let def = baylee_cards::by_index(card)?;
    let face = def
        .faces
        .iter()
        .position(|f| f.name.eq_ignore_ascii_case(name.trim()))
        .unwrap_or(0);
    let english = def.faces.get(face).map_or(name, |f| f.name).to_string();
    Some(CardRef {
        name: english.clone(),
        english,
        card,
        face: u8::try_from(face).unwrap_or(0),
        art: None,
        print: None,
        object: None,
        zone: None,
        owner: None,
        kind: None,
    })
}

/// The bug report's text: what it is, what of the seat's log is
/// `attached`, then each report, as many whole as fit in [`TEXT_CHARS`],
/// and how many more are in `client.reports`.
fn summary(filed: &[&Filed], attached: Option<&Attached<'_>>) -> String {
    let Some(first) = filed.first() else {
        return String::new();
    };
    let mut text = format!(
        "Untrusted: reports written by a language model while it played P{} ({}). \
         Each one is the model's own claim of a rules or engine error; check it against the \
         game's record.\n",
        u16::from(first.seat) + 1,
        minds(filed),
    );
    if let Some(attached) = attached {
        text.push_str(&attached.note());
    }
    // Room is kept for the longest note there could be, so the text never
    // passes its bound however many reports it leaves out.
    let reserve = omitted(filed.len()).chars().count();
    let mut used = text.chars().count();
    for (n, f) in filed.iter().enumerate() {
        let block = block(n + 1, f, attached);
        let size = block.chars().count();
        let last = n + 1 == filed.len();
        if used + size + if last { 0 } else { reserve } > TEXT_CHARS {
            text.push_str(&omitted(filed.len() - n));
            return text;
        }
        text.push_str(&block);
        used += size;
    }
    text
}

/// The minds that filed `filed`, in the order each first did: "model X,
/// profile p", and "; then model Y, …" for a fallback that took over. So
/// the text names the mind of every report, where `client.model` and
/// `client.profile` are the first's and each entry of `client.reports`
/// names its own.
fn minds(filed: &[&Filed]) -> String {
    let mut seen: Vec<(&str, Option<&str>)> = Vec::new();
    for f in filed {
        let mind = (f.model.as_str(), f.profile.as_deref());
        if !seen.contains(&mind) {
            seen.push(mind);
        }
    }
    seen.iter()
        .map(|(model, profile)| match profile {
            Some(profile) => format!("model {model}, profile {profile}"),
            None => format!("model {model}"),
        })
        .collect::<Vec<_>>()
        .join("; then ")
}

/// One report in the text, and after which line of the log `attached` it
/// was made.
fn block(n: usize, filed: &Filed, attached: Option<&Attached<'_>>) -> String {
    let report = &filed.report;
    let at = match attached.and_then(|a| a.line_of(filed)) {
        None => String::new(),
        Some(0) => ", before the log's first line".to_string(),
        Some(line) => format!(", after log line {line}"),
    };
    let mut block = format!(
        "\n{n}. q{}, turn {}{at}: {}\n",
        filed.question, filed.turn, report.what
    );
    if let Some(expected) = &report.expected {
        let _ = writeln!(block, "   Expected: {expected}");
    }
    if let Some(evidence) = &report.evidence {
        let _ = writeln!(block, "   Evidence: {evidence}");
    }
    if !report.references.is_empty() {
        let _ = writeln!(block, "   References: {}", report.references.join(", "));
    }
    if filed.lookups > 0 {
        let _ = writeln!(
            block,
            "   Cards looked up for that question: {}",
            filed.lookups
        );
    }
    block
}

/// The note that ends a text with `left` reports not in it.
fn omitted(left: usize) -> String {
    format!("\n(Shortened: {left} more reports are in client.reports.)\n")
}

#[cfg(test)]
pub(crate) mod tests;
