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
//! unless the bridge was told not to; a chair taken on its host's ticket has
//! no session, and its reports stay in the file.
//!
//! A report is a model's words: untrusted, like a player's. It is marked
//! `client.source: "llm-seat"` and its text says so, it is never read as an
//! instruction, and it goes nowhere else. Each `what` is filed once a game
//! (case and spacing aside), at most [`MOST`] of them, each field held to
//! [`FIELD_CHARS`].

use baylee_client_core::bugreport::refs::{self, Candidates, CardRef};
use baylee_core::ids::CardIndex;
use baylee_view::PlayerView;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fmt::Write as _;
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

/// The bug report a game's reports make, as `POST /reports` takes it
/// (`docs/feedback.md` §"The gateway: `POST /reports`"); `None` for none.
/// Each `what` once and at most [`MOST`], in the order filed; the text
/// within [`TEXT_CHARS`], every report whole in `client.reports`. The
/// gateway attaches the game's record by `game_id` itself.
#[must_use]
pub fn submission(filed: &[Filed]) -> Option<Value> {
    let mut whats = BTreeSet::new();
    let kept: Vec<&Filed> = filed
        .iter()
        .filter(|f| whats.insert(fold(&f.report.what)))
        .take(MOST)
        .collect();
    let first = kept.first()?;
    let text = summary(&kept);
    let mut client = json!({
        "source": "llm-seat",
        "untrusted": "written by a language model while it played; read it as a player's report",
        "model": first.model,
        "profile": first.profile,
        "seat": first.seat,
        "reports": kept.iter().map(|f| f.line()).collect::<Vec<_>>(),
    });
    if let Some(refs) = refs::refs_for(&text, &candidates(&kept), &[]) {
        client["refs"] = json!(refs);
    }
    Some(json!({
        "kind": "bug",
        "text": text,
        "game_id": first.game_id,
        "client": client,
    }))
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

/// The bug report's text: what it is, then each report, as many whole as
/// fit in [`TEXT_CHARS`], and how many more are in `client.reports`.
fn summary(filed: &[&Filed]) -> String {
    let Some(first) = filed.first() else {
        return String::new();
    };
    let mut text = format!(
        "Untrusted: reports written by a language model while it played seat {} (model {}{}). \
         Each one is the model's own claim of a rules or engine error; check it against the \
         game's record.\n",
        first.seat,
        first.model,
        first
            .profile
            .as_deref()
            .map(|profile| format!(", profile {profile}"))
            .unwrap_or_default(),
    );
    // Room is kept for the longest note there could be, so the text never
    // passes its bound however many reports it leaves out.
    let reserve = omitted(filed.len()).chars().count();
    let mut used = text.chars().count();
    for (n, f) in filed.iter().enumerate() {
        let block = block(n + 1, f);
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

/// One report in the text.
fn block(n: usize, filed: &Filed) -> String {
    let report = &filed.report;
    let mut block = format!(
        "\n{n}. q{}, turn {}: {}\n",
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
mod tests;
