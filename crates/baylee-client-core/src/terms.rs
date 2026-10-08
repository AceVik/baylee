//! A gateway's terms of use (WG-1; the shell design, `DESIGN-v5` §11): the
//! Markdown subset the sheet draws, and the sheet's decisions.
//!
//! A gateway may ask its players to accept terms. It says so at sign-in
//! (`terms_stale` on the login and guest answers) and in `/info` (`terms`,
//! the current version, for a guest coming back with a kept session, who
//! never signs in). The client then shows the text in a sheet, enables
//! *Accept and continue* only once the end has been reached, and posts the
//! version it showed (`POST /account/terms`). `docs/protocol.md` §"Terms of
//! use (WG-1)" is the wire.
//!
//! The text is asked for in the interface's language (`GET /terms?lang=`),
//! and asked again when the player switches language while the sheet is up.
//! A gateway that lacks the language answers in English, one with a single
//! file answers that; the version, which is what is accepted, is the same
//! in every language.
//!
//! Two rules the sheet keeps, both here so they are tested without a
//! renderer: **Esc never signs out** — it only moves the keyboard's focus to
//! *Not now* (the sheet is modal and cannot be dismissed without choosing);
//! and **a guest is asked before Not now signs it out**, because a guest
//! that signs out loses its decks.
//!
//! The renderer is deliberately small: `#`/`##` headings, paragraphs,
//! `**bold**`, `*italic*`/`_italic_`, `-`/`*` and `1.` lists, `---` rules,
//! and a link drawn as its text and its address (`[text](url)`,
//! `<url>`). Anything else is text. No HTML is interpreted, no image is
//! fetched and no link is opened from here: the text is the gateway's, and
//! a hostile gateway gets plain words on a sheet and nothing more.

use serde::Deserialize;

use crate::i18n::Lang;

/// A run of text in one style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    /// The words.
    pub text: String,
    /// `**bold**`.
    pub bold: bool,
    /// `*italic*` or `_italic_`.
    pub italic: bool,
}

impl Span {
    fn plain(text: &str) -> Self {
        Self {
            text: text.to_string(),
            bold: false,
            italic: false,
        }
    }
}

/// One block of the text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    /// `#` (1) or `##` and deeper (2).
    Heading {
        /// 1 or 2.
        level: u8,
        /// Its words.
        spans: Vec<Span>,
    },
    /// Lines joined until a blank line.
    Paragraph(Vec<Span>),
    /// One list item: `-`/`*` (no number) or `1.` (its number).
    Item {
        /// The number of an ordered item.
        number: Option<u32>,
        /// Its words.
        spans: Vec<Span>,
    },
    /// `---`.
    Rule,
}

/// The text of `markdown` as blocks.
#[must_use]
pub fn parse(markdown: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let flush = |paragraph: &mut Vec<&str>, blocks: &mut Vec<Block>| {
        if !paragraph.is_empty() {
            blocks.push(Block::Paragraph(inline(&paragraph.join(" "))));
            paragraph.clear();
        }
    };
    for raw in markdown.lines() {
        let line = raw.trim();
        // A comment line (the version and date the gateway already read).
        if line.starts_with("<!--") && line.ends_with("-->") {
            flush(&mut paragraph, &mut blocks);
            continue;
        }
        if line.is_empty() {
            flush(&mut paragraph, &mut blocks);
            continue;
        }
        if is_rule(line) {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Rule);
            continue;
        }
        if let Some((level, rest)) = heading(line) {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Heading {
                level,
                spans: inline(rest),
            });
            continue;
        }
        if let Some(rest) = line
            .strip_prefix("- ")
            .or_else(|| line.strip_prefix("* "))
            .or_else(|| line.strip_prefix("+ "))
        {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Item {
                number: None,
                spans: inline(rest.trim()),
            });
            continue;
        }
        if let Some((number, rest)) = ordered(line) {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Item {
                number: Some(number),
                spans: inline(rest),
            });
            continue;
        }
        // A line under a list item that is indented continues it.
        if raw.starts_with([' ', '\t'])
            && paragraph.is_empty()
            && let Some(Block::Item { spans, .. }) = blocks.last_mut()
        {
            for span in std::iter::once(Span::plain(" ")).chain(inline(line)) {
                match spans.last_mut() {
                    Some(last) if last.bold == span.bold && last.italic == span.italic => {
                        last.text.push_str(&span.text);
                    }
                    _ => spans.push(span),
                }
            }
            continue;
        }
        paragraph.push(line);
    }
    flush(&mut paragraph, &mut blocks);
    blocks
}

fn is_rule(line: &str) -> bool {
    let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    compact.len() >= 3
        && (compact.chars().all(|c| c == '-')
            || compact.chars().all(|c| c == '*')
            || compact.chars().all(|c| c == '_'))
}

fn heading(line: &str) -> Option<(u8, &str)> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = line[hashes..].strip_prefix(' ')?;
    Some((
        if hashes == 1 { 1 } else { 2 },
        rest.trim().trim_end_matches('#').trim(),
    ))
}

fn ordered(line: &str) -> Option<(u32, &str)> {
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 || digits > 9 {
        return None;
    }
    let rest = line[digits..]
        .strip_prefix(". ")
        .or_else(|| line[digits..].strip_prefix(") "))?;
    Some((line[..digits].parse().ok()?, rest.trim()))
}

/// The inline styles of one line: bold, italic, links, escapes.
#[must_use]
pub fn inline(text: &str) -> Vec<Span> {
    let mut spans: Vec<Span> = Vec::new();
    let mut bold = false;
    let mut italic = false;
    let mut word = String::new();
    let push = |spans: &mut Vec<Span>, word: &mut String, bold: bool, italic: bool| {
        if word.is_empty() {
            return;
        }
        match spans.last_mut() {
            Some(last) if last.bold == bold && last.italic == italic => last.text.push_str(word),
            _ => spans.push(Span {
                text: word.clone(),
                bold,
                italic,
            }),
        }
        word.clear();
    };
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' if i + 1 < chars.len() && chars[i + 1].is_ascii_punctuation() => {
                word.push(chars[i + 1]);
                i += 2;
            }
            '*' if chars.get(i + 1) == Some(&'*') => {
                push(&mut spans, &mut word, bold, italic);
                bold = !bold;
                i += 2;
            }
            // `_` inside a word (`snake_case`) is a letter, not a style.
            '*' | '_'
                if c == '*'
                    || !(i > 0
                        && chars[i - 1].is_alphanumeric()
                        && chars.get(i + 1).is_some_and(|n| n.is_alphanumeric())) =>
            {
                push(&mut spans, &mut word, bold, italic);
                italic = !italic;
                i += 1;
            }
            '[' => {
                if let Some((label, url, next)) = link_at(&chars, i) {
                    word.push_str(&label);
                    if !url.is_empty() && url != label {
                        word.push_str(" (");
                        word.push_str(&url);
                        word.push(')');
                    }
                    i = next;
                } else {
                    word.push(c);
                    i += 1;
                }
            }
            '<' => {
                if let Some(end) = chars[i + 1..].iter().position(|c| *c == '>') {
                    let inner: String = chars[i + 1..i + 1 + end].iter().collect();
                    if inner.starts_with("http://")
                        || inner.starts_with("https://")
                        || inner.starts_with("mailto:")
                    {
                        word.push_str(&inner);
                        i += end + 2;
                        continue;
                    }
                }
                word.push(c);
                i += 1;
            }
            _ => {
                word.push(c);
                i += 1;
            }
        }
    }
    push(&mut spans, &mut word, bold, italic);
    spans
}

/// `[label](url)` starting at `at`: the label, the address and where the
/// text goes on after it.
fn link_at(chars: &[char], at: usize) -> Option<(String, String, usize)> {
    let close = at + 1 + chars[at + 1..].iter().position(|c| *c == ']')?;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = close + 2 + chars[close + 2..].iter().position(|c| *c == ')')?;
    let label: String = chars[at + 1..close].iter().collect();
    let url: String = chars[close + 2..end].iter().collect();
    Some((label, url.trim().to_string(), end + 1))
}

/// What `GET /terms` answers.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct TermsDoc {
    /// The version the gateway asks to be accepted.
    pub version: String,
    /// The date the operator wrote on it, if any.
    #[serde(default)]
    pub updated: Option<String>,
    /// The text.
    pub markdown: String,
    /// The language the text is in, when the gateway has the terms in more
    /// than one (the asked one, or English where it lacks that); `None` from
    /// a gateway with one text for all.
    #[serde(default)]
    pub lang: Option<String>,
}

/// Where the terms are asked for: `GET /terms` in the interface's language.
#[must_use]
pub fn url(gateway: &str, lang: Lang) -> String {
    format!(
        "{}/terms?lang={}",
        gateway.trim_end_matches('/'),
        lang.code()
    )
}

/// Whether the sheet has to be shown after a sign-in.
///
/// `stale` is the sign-in's `terms_stale`: the gateway's own answer, and it
/// decides when there is one. Without one — a guest coming back with the
/// session this device kept, which signs in to nothing — the version `/info`
/// names is held against the one this device last accepted there.
#[must_use]
pub fn must_ask(stale: Option<bool>, current: Option<&str>, accepted_here: Option<&str>) -> bool {
    match stale {
        Some(stale) => stale,
        None => current.is_some_and(|current| accepted_here != Some(current)),
    }
}

/// What a press on the sheet asks the shell to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    /// `GET /terms` in this language ([`url`]).
    Fetch(Lang),
    /// `POST /account/terms {version}`.
    Accept(String),
    /// Sign out, nothing stored.
    SignOut,
}

/// Where the sheet stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Sheet {
    /// Not up.
    #[default]
    Closed,
    /// Asked for; the sheet is up with nothing in it yet.
    Fetching,
    /// `GET /terms` failed: "Couldn't load the terms · Retry". Nothing is
    /// accepted and the session stays.
    Failed,
    /// The text is up.
    Reading(Reading),
}

/// The text on the sheet and what the player has done with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reading {
    /// What the gateway served.
    pub doc: TermsDoc,
    /// Its blocks.
    pub blocks: Vec<Block>,
    /// The end has been seen (scrolled to, or the text fits), which is what
    /// enables Accept. Never taken back.
    pub read_to_end: bool,
    /// The acceptance is on its way.
    pub sending: bool,
    /// A guest pressed Not now and is being asked whether to sign out.
    pub asking_guest: bool,
}

/// The sheet's decisions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Terms {
    sheet: Sheet,
    /// The language the text was last asked in: an answer asked in another
    /// is one the player has switched away from, and is dropped.
    lang: Lang,
}

impl Terms {
    /// Where the sheet stands.
    #[must_use]
    pub fn sheet(&self) -> &Sheet {
        &self.sheet
    }

    /// Whether the sheet is up (it holds the screen until answered).
    #[must_use]
    pub fn up(&self) -> bool {
        !matches!(self.sheet, Sheet::Closed)
    }

    /// The sheet is wanted: it goes up and the text is asked for in the
    /// interface's language.
    pub fn ask(&mut self, lang: Lang) -> Ask {
        self.sheet = Sheet::Fetching;
        self.lang = lang;
        Ask::Fetch(lang)
    }

    /// The language the text was last asked in.
    #[must_use]
    pub fn lang(&self) -> Lang {
        self.lang
    }

    /// The player switched the interface's language: the sheet, if it is up,
    /// asks for the text again in the new one. The text on it stays until the
    /// new one arrives, and must then be read to its end again, since it is
    /// another text. Nothing while the acceptance is on its way (the sheet is
    /// about to close), nor when the language is the one already asked.
    pub fn relang(&mut self, lang: Lang) -> Option<Ask> {
        if !self.up() || lang == self.lang {
            return None;
        }
        match &self.sheet {
            Sheet::Reading(reading) if reading.sending => None,
            Sheet::Reading(_) => {
                self.lang = lang;
                Some(Ask::Fetch(lang))
            }
            _ => Some(self.ask(lang)),
        }
    }

    /// `GET /terms` answered, to a question asked in `asked`.
    pub fn loaded(&mut self, doc: TermsDoc, asked: Lang) {
        // The sheet went down meanwhile, or the player switched language and
        // the answer to the new question is still on its way.
        if !self.up() || asked != self.lang {
            return;
        }
        let blocks = parse(&doc.markdown);
        self.sheet = Sheet::Reading(Reading {
            doc,
            blocks,
            read_to_end: false,
            sending: false,
            asking_guest: false,
        });
    }

    /// `GET /terms` failed, or the gateway has no terms after all.
    pub fn failed(&mut self) {
        if self.up() {
            self.sheet = Sheet::Failed;
        }
    }

    /// Retry after a failure.
    pub fn retry(&mut self) -> Option<Ask> {
        matches!(self.sheet, Sheet::Failed).then(|| self.ask(self.lang))
    }

    /// The end of the text is in view: scrolled there, `End`, or the text
    /// fits without scrolling. Accept is enabled from now on.
    pub fn reached_end(&mut self) {
        if let Sheet::Reading(reading) = &mut self.sheet
            && !reading.read_to_end
        {
            reading.read_to_end = true;
        }
    }

    /// Whether Accept works.
    #[must_use]
    pub fn can_accept(&self) -> bool {
        matches!(&self.sheet, Sheet::Reading(r) if r.read_to_end && !r.sending && !r.asking_guest)
    }

    /// Accept and continue: the version shown goes to the gateway. Nothing
    /// until the end has been reached.
    pub fn accept(&mut self) -> Option<Ask> {
        if !self.can_accept() {
            return None;
        }
        let Sheet::Reading(reading) = &mut self.sheet else {
            return None;
        };
        reading.sending = true;
        Some(Ask::Accept(reading.doc.version.clone()))
    }

    /// The gateway recorded it: the sheet closes. The version, for the
    /// device's own copy.
    pub fn accepted(&mut self) -> Option<String> {
        let Sheet::Reading(reading) = &self.sheet else {
            return None;
        };
        let version = reading.doc.version.clone();
        self.sheet = Sheet::Closed;
        Some(version)
    }

    /// The gateway refused the version (`409`: the file changed while the
    /// sheet was up): the new text is asked for and must be read again.
    pub fn changed(&mut self) -> Ask {
        self.ask(self.lang)
    }

    /// The acceptance failed some other way: Accept works again.
    pub fn send_failed(&mut self) {
        if let Sheet::Reading(reading) = &mut self.sheet {
            reading.sending = false;
        }
    }

    /// Not now. An account signs out at once, nothing stored; a guest is
    /// asked first, since a guest that signs out loses its decks.
    pub fn not_now(&mut self, guest: bool) -> Option<Ask> {
        match &mut self.sheet {
            Sheet::Reading(reading) if guest && !reading.asking_guest => {
                reading.asking_guest = true;
                None
            }
            Sheet::Closed => None,
            _ => {
                self.sheet = Sheet::Closed;
                Some(Ask::SignOut)
            }
        }
    }

    /// The guest answered "Stay": back to the text.
    pub fn stay(&mut self) {
        if let Sheet::Reading(reading) = &mut self.sheet {
            reading.asking_guest = false;
        }
    }

    /// Esc: nothing destructive (M4-1). A guest's question is answered with
    /// its safe answer (stay); otherwise nothing changes here, and the shell
    /// moves the keyboard's focus to Not now.
    pub fn escape(&mut self) {
        self.stay();
    }

    /// The session went (signed out elsewhere, expired): the sheet goes with
    /// it, nothing stored.
    pub fn close(&mut self) {
        self.sheet = Sheet::Closed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> TermsDoc {
        TermsDoc {
            version: "v1".into(),
            updated: Some("2026-10-07".into()),
            markdown: "# Terms\n\nPlay **fair**.".into(),
            lang: None,
        }
    }

    fn reading() -> Terms {
        let mut terms = Terms::default();
        assert_eq!(terms.ask(Lang::En), Ask::Fetch(Lang::En));
        terms.loaded(doc(), Lang::En);
        terms
    }

    #[test]
    fn the_subset_reads_as_blocks() {
        let text = "<!-- version: x -->\n# Title\n\nOne line\nand the next.\n\n## Part *two*\n\n- a\n* b\n  continued\n\n1. first\n2) second\n\n---\n\nEnd.";
        let blocks = parse(text);
        assert_eq!(
            blocks,
            vec![
                Block::Heading {
                    level: 1,
                    spans: vec![Span::plain("Title")]
                },
                Block::Paragraph(vec![Span::plain("One line and the next.")]),
                Block::Heading {
                    level: 2,
                    spans: vec![
                        Span::plain("Part "),
                        Span {
                            text: "two".into(),
                            bold: false,
                            italic: true
                        }
                    ]
                },
                Block::Item {
                    number: None,
                    spans: vec![Span::plain("a")]
                },
                Block::Item {
                    number: None,
                    spans: vec![Span::plain("b continued")]
                },
                Block::Item {
                    number: Some(1),
                    spans: vec![Span::plain("first")]
                },
                Block::Item {
                    number: Some(2),
                    spans: vec![Span::plain("second")]
                },
                Block::Rule,
                Block::Paragraph(vec![Span::plain("End.")]),
            ]
        );
    }

    #[test]
    fn bold_italic_links_and_escapes() {
        assert_eq!(
            inline("a **b** *c* _d_ ***e***"),
            vec![
                Span::plain("a "),
                Span {
                    text: "b".into(),
                    bold: true,
                    italic: false
                },
                Span::plain(" "),
                Span {
                    text: "c".into(),
                    bold: false,
                    italic: true
                },
                Span::plain(" "),
                Span {
                    text: "d".into(),
                    bold: false,
                    italic: true
                },
                Span::plain(" "),
                Span {
                    text: "e".into(),
                    bold: true,
                    italic: true
                },
            ]
        );
        assert_eq!(
            inline("see [the rules](https://x.invalid/r) or <https://y.invalid>"),
            vec![Span::plain(
                "see the rules (https://x.invalid/r) or https://y.invalid"
            )]
        );
        assert_eq!(
            inline(r"\*not\* snake_case"),
            vec![Span::plain("*not* snake_case")]
        );
        // HTML is words, never markup.
        assert_eq!(inline("<b>x</b>"), vec![Span::plain("<b>x</b>")]);
    }

    /// The repository's placeholder renders without a stray marker.
    #[test]
    fn the_placeholder_renders_cleanly() {
        let text = include_str!("../../../docs/terms-placeholder.md");
        let blocks = parse(text);
        assert!(blocks.len() > 8, "{blocks:?}");
        for block in &blocks {
            let spans = match block {
                Block::Heading { spans, .. }
                | Block::Paragraph(spans)
                | Block::Item { spans, .. } => spans,
                Block::Rule => continue,
            };
            for span in spans {
                assert!(
                    !span.text.contains("**") && !span.text.contains("<!--"),
                    "{span:?}"
                );
            }
        }
        assert!(blocks.contains(&Block::Rule));
        assert!(blocks.iter().any(|b| matches!(
            b,
            Block::Item {
                number: Some(2),
                ..
            }
        )));
        // The per-language placeholders, likewise.
        for text in [
            include_str!("../../../docs/terms-placeholder/terms.de.md"),
            include_str!("../../../docs/terms-placeholder/terms.en.md"),
        ] {
            let blocks = parse(text);
            assert!(blocks.contains(&Block::Rule), "{blocks:?}");
            assert!(
                blocks.iter().all(|b| match b {
                    Block::Heading { spans, .. }
                    | Block::Paragraph(spans)
                    | Block::Item { spans, .. } => spans
                        .iter()
                        .all(|s| !s.text.contains("**") && !s.text.contains("<!--")),
                    Block::Rule => true,
                }),
                "{blocks:?}"
            );
        }
    }

    #[test]
    fn accept_waits_for_the_end_and_sends_the_version_shown() {
        let mut terms = reading();
        assert!(!terms.can_accept());
        assert_eq!(terms.accept(), None, "not before the end");
        terms.reached_end();
        assert_eq!(terms.accept(), Some(Ask::Accept("v1".into())));
        assert_eq!(terms.accept(), None, "once");
        assert_eq!(terms.accepted(), Some("v1".into()));
        assert!(!terms.up());
    }

    #[test]
    fn a_changed_text_must_be_read_again() {
        let mut terms = reading();
        terms.reached_end();
        terms.accept();
        assert_eq!(terms.changed(), Ask::Fetch(Lang::En));
        terms.loaded(
            TermsDoc {
                version: "v2".into(),
                ..doc()
            },
            Lang::En,
        );
        assert!(!terms.can_accept(), "the new text is unread");
    }

    /// M4-1: Esc ×10 on the sheet changes nothing — still up, nothing sent,
    /// nothing signed out.
    #[test]
    fn escape_ten_times_leaves_the_sheet_up_and_nothing_sent() {
        let mut terms = reading();
        let before = terms.clone();
        for _ in 0..10 {
            terms.escape();
        }
        assert_eq!(terms, before);
        assert!(terms.up());
        // On a guest's question Esc is the safe answer: stay.
        terms.not_now(true);
        terms.escape();
        assert_eq!(terms, before);
    }

    #[test]
    fn a_guest_is_asked_before_not_now_signs_it_out() {
        let mut guest = reading();
        assert_eq!(guest.not_now(true), None, "asked first");
        assert!(matches!(guest.sheet(), Sheet::Reading(r) if r.asking_guest));
        guest.reached_end();
        assert!(!guest.can_accept(), "not while the question stands");
        guest.stay();
        assert!(guest.can_accept());
        guest.not_now(true);
        assert_eq!(guest.not_now(true), Some(Ask::SignOut), "the second press");
        assert!(!guest.up());

        let mut account = reading();
        assert_eq!(account.not_now(false), Some(Ask::SignOut), "at once");
    }

    #[test]
    fn a_failed_fetch_offers_retry_and_accepts_nothing() {
        let mut terms = Terms::default();
        terms.ask(Lang::De);
        terms.failed();
        assert_eq!(terms.sheet(), &Sheet::Failed);
        assert_eq!(terms.accept(), None);
        assert_eq!(terms.retry(), Some(Ask::Fetch(Lang::De)), "in its language");
        // Not now still signs out from a failed sheet.
        terms.failed();
        assert_eq!(terms.not_now(false), Some(Ask::SignOut));
    }

    /// The request carries the interface's language, so a gateway with the
    /// terms in several shows the player's.
    #[test]
    fn the_terms_are_asked_for_in_the_interface_language() {
        assert_eq!(
            url("http://gw.example:28766/", Lang::De),
            "http://gw.example:28766/terms?lang=de"
        );
        assert_eq!(url("https://gw", Lang::En), "https://gw/terms?lang=en");
        let mut terms = Terms::default();
        assert_eq!(terms.ask(Lang::De), Ask::Fetch(Lang::De));
        assert_eq!(terms.lang(), Lang::De);
        // An older gateway, or one file for all, names no language.
        let doc: TermsDoc =
            serde_json::from_str(r#"{"version":"v1","markdown":"x"}"#).expect("parses");
        assert_eq!(doc.lang, None);
        let doc: TermsDoc =
            serde_json::from_str(r#"{"version":"v1","markdown":"x","lang":"de"}"#).expect("parses");
        assert_eq!(doc.lang.as_deref(), Some("de"));
    }

    /// A language switch while the sheet is up asks again; the text on it
    /// stays until the new one comes, which must then be read to its end;
    /// an answer to the language switched away from is dropped.
    #[test]
    fn switching_language_asks_again_and_drops_the_old_answer() {
        let mut terms = reading();
        terms.reached_end();
        assert_eq!(terms.relang(Lang::En), None, "the language it has");
        assert_eq!(terms.relang(Lang::De), Some(Ask::Fetch(Lang::De)));
        assert!(terms.can_accept(), "the old text stays until the new comes");
        // The English answer, late: not what is now wanted.
        terms.loaded(doc(), Lang::En);
        assert!(terms.can_accept(), "a stale answer replaced the sheet");
        let german = TermsDoc {
            markdown: "# Bedingungen".into(),
            lang: Some("de".into()),
            ..doc()
        };
        terms.loaded(german.clone(), Lang::De);
        assert!(matches!(terms.sheet(), Sheet::Reading(r) if r.doc == german));
        assert!(!terms.can_accept(), "another text, unread");

        // While the text is still coming, or failed, the switch asks again.
        let mut fetching = Terms::default();
        fetching.ask(Lang::En);
        assert_eq!(fetching.relang(Lang::De), Some(Ask::Fetch(Lang::De)));
        assert_eq!(fetching.sheet(), &Sheet::Fetching);
        fetching.failed();
        assert_eq!(fetching.relang(Lang::En), Some(Ask::Fetch(Lang::En)));
        assert_eq!(fetching.sheet(), &Sheet::Fetching);

        // Nothing while the acceptance is on its way, or the sheet is down.
        let mut sending = reading();
        sending.reached_end();
        sending.accept();
        assert_eq!(sending.relang(Lang::De), None);
        sending.send_failed();
        assert_eq!(sending.relang(Lang::De), Some(Ask::Fetch(Lang::De)));
        assert_eq!(Terms::default().relang(Lang::De), None);
    }

    #[test]
    fn who_is_asked() {
        assert!(must_ask(Some(true), None, None));
        assert!(
            !must_ask(Some(false), Some("v1"), None),
            "the gateway decides"
        );
        // A kept guest: the device's copy against `/info`.
        assert!(must_ask(None, Some("v1"), None));
        assert!(must_ask(None, Some("v2"), Some("v1")));
        assert!(!must_ask(None, Some("v1"), Some("v1")));
        assert!(!must_ask(None, None, Some("v1")), "a gateway without terms");
    }
}
