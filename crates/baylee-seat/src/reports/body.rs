//! The bug reports a game's reports make, as the gateway takes them
//! (`docs/feedback.md` §"The gateway: `POST /reports`"): one a report
//! ([`submissions`]), or all of them in one where a route takes one a game
//! ([`bundle`]).

use super::checks::{ANSWER_CHARS, ID_CHARS, cut, hex};
use super::read::Kind;
use super::{CLIENT_BYTES, Filed, LOG_RESERVE, MOST, TEXT_CHARS};
use baylee_client_core::bugreport::refs::{self, Candidates, CardRef};
use baylee_client_core::bugreport::{LogRow, SeatLog};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as _;

/// The most lines of the seat's log a report's `record_tail` keeps: the
/// last before the report.
pub const RECORD_TAIL_LINES: usize = 20;

/// The longest a line of `record_tail` is kept, in characters.
pub const RECORD_TAIL_CHARS: usize = 300;

/// One bug report a report, as `POST /reports` takes it, in the order
/// filed: each dedupe key once and at most [`MOST`]
/// ([`super::Checked::dedupe_key`]); empty for none. Each one's text is
/// within [`TEXT_CHARS`] and its report whole in `client.report`, beside
/// the checks the seat made flat in it, so triage filters one report at a
/// time. The game's record is the gateway's to attach, by `game_id`; the
/// seat attaches none of its own (`local_record`), and `client` names the
/// game and carries `record_sha256` and `record_tail` instead.
///
/// With `log` ([`super::game_log`]), the seat's log rides in each as
/// `client.log`, in the desktop client's shape, and `client.log_omitted`
/// says how many of its oldest lines were left out so that `client` stays
/// within [`CLIENT_BYTES`] (0: none); the text says so too, and the
/// report, in the text and as `decision.log_line`, after which line of the
/// log as sent it was made (from 1, as the feedback service lists them; 0
/// before the first).
#[must_use]
pub fn submissions(filed: &[Filed], log: Option<&SeatLog>) -> Vec<Value> {
    let kept = kept(filed);
    let record = log.map(Record::of);
    kept.iter()
        .map(|f| {
            let tail = record.as_ref().map(|record| record.tail(f));
            let sha = record.as_ref().map(|record| record.sha256.as_str());
            fitted(log.cloned(), |attached| {
                single(f, attached, sha, tail.as_ref())
            })
        })
        .collect()
}

/// A game's reports in one bug report, for a route that takes one a game
/// (a chair on its host's ticket, `POST /lobby/games/{id}/chair/report`):
/// every report in `client.reports`, each the object [`submissions`] puts
/// in `client.report` with its own `record_tail`; one report alone goes as
/// [`submissions`] makes it. `None` for none. Should the reports alone
/// pass [`CLIENT_BYTES`], the last ones are left out of `client.reports`
/// and `client.reports_omitted` counts them; the reports file keeps them
/// all.
#[must_use]
pub fn bundle(filed: &[Filed], log: Option<&SeatLog>) -> Option<Value> {
    let kept = kept(filed);
    if kept.len() <= 1 {
        return submissions(filed, log).pop();
    }
    let record = log.map(Record::of);
    let tails: Vec<Option<Value>> = kept
        .iter()
        .map(|f| record.as_ref().map(|record| record.tail(f)))
        .collect();
    let sha = record.as_ref().map(|record| record.sha256.as_str());
    // As many reports as leave room for the log's place in `client`.
    let mut sent = kept.len();
    while sent > 1
        && weight(&many(&kept[..sent], kept.len() - sent, None, sha, &tails)["client"])
            > CLIENT_BYTES.saturating_sub(LOG_RESERVE)
    {
        sent -= 1;
    }
    let left = kept.len() - sent;
    Some(fitted(log.cloned(), |attached| {
        many(&kept[..sent], left, attached, sha, &tails)
    }))
}

/// How many of `filed` go to the gateway: each dedupe key once, at most
/// [`MOST`].
#[must_use]
pub fn count(filed: &[Filed]) -> usize {
    kept(filed).len()
}

/// The reports of `filed` that are filed: each dedupe key once, at most
/// [`MOST`], in the order filed.
fn kept(filed: &[Filed]) -> Vec<&Filed> {
    let mut keys = BTreeSet::new();
    filed
        .iter()
        .filter(|f| keys.insert(f.checked.dedupe_key.as_str()))
        .take(MOST)
        .collect()
}

/// The body `build` makes, with as much of `log` as keeps `client` within
/// [`CLIENT_BYTES`]: the whole log, else its newest lines, else none of
/// its lines.
fn fitted(log: Option<SeatLog>, build: impl Fn(Option<&Attached<'_>>) -> Value) -> Value {
    let Some(mut log) = log else {
        return build(None);
    };
    let whole = build(Some(&Attached::new(&log, 0)));
    if weight(&whole["client"]) <= CLIENT_BYTES {
        return whole;
    }
    // The room the rest leaves, less what a shorter log may change in it.
    let mut rest = whole;
    if let Some(client) = rest["client"].as_object_mut() {
        client.remove("log");
    }
    let room = CLIENT_BYTES.saturating_sub(weight(&rest["client"]).saturating_add(LOG_RESERVE));
    let lines = log.lines.len();
    let omitted = log.fit(room);
    let fitted = build(Some(&Attached::new(&log, omitted)));
    if weight(&fitted["client"]) <= CLIENT_BYTES {
        return fitted;
    }
    // Never for reports held to their bounds; were it so, the reports go
    // without the log's lines rather than not at all.
    log.lines.clear();
    build(Some(&Attached::new(&log, lines)))
}

/// What a report carries of the seat's whole log, before any of it is left
/// out to fit: the log's hash, and its lines for each report's tail.
struct Record<'a> {
    /// `record_sha256`: SHA-256, in hex, of every line's text followed by a
    /// line break, in order.
    sha256: String,
    lines: &'a [LogRow],
}

impl<'a> Record<'a> {
    fn of(log: &'a SeatLog) -> Self {
        let mut hash = Sha256::new();
        for line in &log.lines {
            hash.update(line.text.as_bytes());
            hash.update(b"\n");
        }
        Self {
            sha256: hex(&hash.finalize()),
            lines: &log.lines,
        }
    }

    /// `record_tail`: the last [`RECORD_TAIL_LINES`] lines of the log the
    /// seat had been told when `filed`'s question was asked, each
    /// `{turn, text}`, its text cut to [`RECORD_TAIL_CHARS`].
    fn tail(&self, filed: &Filed) -> Value {
        let end = usize::try_from(filed.log_lines)
            .unwrap_or(usize::MAX)
            .min(self.lines.len());
        let start = end.saturating_sub(RECORD_TAIL_LINES);
        self.lines[start..end]
            .iter()
            .map(|line| json!({"turn": line.turn, "text": cut(&line.text, RECORD_TAIL_CHARS)}))
            .collect()
    }
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

/// What every bug report of `first`'s game says in `client` of where its
/// reports come from, with the log as `attached` and the hash of the whole
/// log, `sha`.
fn client(first: &Filed, attached: Option<&Attached<'_>>, sha: Option<&str>) -> Value {
    let mut client = json!({
        "source": first.source.as_str(),
        "untrusted": first.source.untrusted(),
        "model": first.model,
        "profile": first.profile,
        "seat": first.seat,
        "game_id": first.game_id,
    });
    if let Some(sha) = sha {
        client["record_sha256"] = json!(sha);
    }
    if let Some(attached) = attached {
        client["log"] = json!(attached.log);
        client["log_omitted"] = json!(attached.omitted);
    }
    client
}

/// The bug report of the one report `f`, with the log as `attached`, the
/// whole log's hash `sha` and the log's lines before it, `tail`.
fn single(
    f: &Filed,
    attached: Option<&Attached<'_>>,
    sha: Option<&str>,
    tail: Option<&Value>,
) -> Value {
    let text = summary(&[f], attached);
    let mut client = client(f, attached, sha);
    client["report"] = f.value(attached.and_then(|a| a.line_of(f)));
    if let Some(tail) = tail {
        client["record_tail"] = tail.clone();
    }
    if let Some(refs) = refs::refs_for(&text, &candidates(&[f]), &[]) {
        client["refs"] = json!(refs);
    }
    json!({
        "kind": "bug",
        "text": text,
        "game_id": f.game_id,
        "client": client,
    })
}

/// The bug report of the reports `kept` (at least one) in one, `left` more
/// left out of it, with the log as `attached`, the whole log's hash `sha`
/// and each report's tail of it among `tails` (by its place in `kept`).
fn many(
    kept: &[&Filed],
    left: usize,
    attached: Option<&Attached<'_>>,
    sha: Option<&str>,
    tails: &[Option<Value>],
) -> Value {
    let first = kept[0];
    let text = summary(kept, attached);
    let reports: Vec<Value> = kept
        .iter()
        .zip(tails)
        .map(|(f, tail)| {
            let mut report = f.value(attached.and_then(|a| a.line_of(f)));
            report["model"] = json!(f.model);
            report["profile"] = json!(f.profile);
            if let Some(tail) = tail {
                report["record_tail"] = tail.clone();
            }
            report
        })
        .collect();
    // `model` and `profile` are the mind that filed the first report; after
    // a fallback took over each report names its own, and the text every
    // mind in order ([`minds`]).
    let mut client = client(first, attached, sha);
    client["reports"] = json!(reports);
    if left > 0 {
        client["reports_omitted"] = json!(left);
    }
    if let Some(refs) = refs::refs_for(&text, &candidates(kept), &[]) {
        client["refs"] = json!(refs);
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

/// A bug report's text: what it is, what of the seat's log is `attached`,
/// then each report, as many whole as fit in [`TEXT_CHARS`], and where the
/// rest is. One report is told without a number.
fn summary(filed: &[&Filed], attached: Option<&Attached<'_>>) -> String {
    let Some(first) = filed.first() else {
        return String::new();
    };
    let one = filed.len() == 1;
    let (what, each) = if one {
        ("a report", "It is")
    } else {
        ("reports", "Each one is")
    };
    let mut text = format!(
        "Untrusted: {what} written by a language model while it {} P{} ({}). {each} the \
         model's own claim of a rules or engine error: check it against the Comprehensive \
         Rules, the card's Oracle text and the game's record.\n",
        first.source.did(),
        u16::from(first.seat) + 1,
        minds(filed),
    );
    if let Some(attached) = attached {
        text.push_str(&attached.note());
    }
    // Room is kept for the longest note there could be, so the text never
    // passes its bound however many reports it leaves out.
    let reserve = omitted(filed.len(), one).chars().count();
    let mut used = text.chars().count();
    for (n, f) in filed.iter().enumerate() {
        let block = block((!one).then_some(n + 1), f, attached);
        let size = block.chars().count();
        let last = n + 1 == filed.len();
        if used + size + if last { 0 } else { reserve } > TEXT_CHARS {
            text.push_str(&omitted(filed.len() - n, one));
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

/// One report in the text, numbered `n` among several, and after which
/// line of the log `attached` it was made.
fn block(n: Option<usize>, filed: &Filed, attached: Option<&Attached<'_>>) -> String {
    let report = &filed.report;
    let checked = &filed.checked;
    let at = match attached.and_then(|a| a.line_of(filed)) {
        None => String::new(),
        Some(0) => ", before the log's first line".to_string(),
        Some(line) => format!(", after log line {line}"),
    };
    let mut kind = checked.kind.as_str().to_string();
    if checked.kind == Kind::Unverified {
        let _ = write!(
            kind,
            " (the model said {}; it quoted no three words of a card's rules text as it was \
             told them and cited no rule number spelt as one)",
            report.kind.as_str()
        );
    }
    if let Some(severity) = report.severity {
        let _ = write!(kind, ", {}", severity.as_str());
    }
    if let Some(confidence) = report.confidence {
        let _ = write!(kind, ", confidence {}", confidence.as_str());
    }
    let number = n.map(|n| format!("{n}. ")).unwrap_or_default();
    let mut block = format!(
        "\n{number}q{}, turn {}{at}: {kind}\n   Observed: {}\n",
        filed.question, filed.moment.turn, report.observed
    );
    if let Some(expected) = &report.expected {
        let _ = writeln!(block, "   Expected: {expected}");
    }
    if let Some(evidence) = &report.evidence {
        let _ = writeln!(block, "   Evidence: {evidence}");
    }
    for card in &report.cards {
        match &card.oracle_quote {
            Some(quote) => {
                let _ = writeln!(block, "   Card: [{}] \"{quote}\"", card.name);
            }
            None => {
                let _ = writeln!(block, "   Card: [{}]", card.name);
            }
        }
    }
    if !report.rules.is_empty() {
        let _ = writeln!(block, "   Rules: {}", report.rules.join(", "));
    }
    let yes = |ok: bool| if ok { "yes" } else { "no" };
    let mut checks = Vec::new();
    if let Some(matches) = checked.oracle_quote_matches {
        checks.push(format!(
            "every quote is words of the card text the model was told: {}",
            yes(matches)
        ));
    }
    if let Some(ok) = checked.rules_format_ok {
        checks.push(format!("every rule number is spelt as one: {}", yes(ok)));
    }
    if !checks.is_empty() {
        let _ = writeln!(block, "   The seat checked: {}", checks.join("; "));
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
    // Held to the bounds `decision` keeps them to (`Moment::value`): a
    // refusal repeats the model's words, and one long answer would push
    // the whole block out of the text.
    if let Some(answer) = &filed.moment.answer {
        let _ = writeln!(
            block,
            "   The question: {}; the answer: {}",
            cut(&filed.moment.pending, ID_CHARS),
            cut(answer, ANSWER_CHARS)
        );
    }
    block
}

/// The note that ends a text with `left` reports not in it, or the rest of
/// the `one` report it tells.
fn omitted(left: usize, one: bool) -> String {
    if one {
        "\n(Shortened: the whole report is in client.report.)\n".to_string()
    } else {
        format!("\n(Shortened: {left} more reports are in client.reports.)\n")
    }
}
