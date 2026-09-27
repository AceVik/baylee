//! The report form's state, and what each answer of the gateway means.
//!
//! `POST {gateway}/reports` answers `201 {"report_id"}`, or refuses with one
//! of five statuses the player has to be told apart (#309): a bad request
//! (the gateway's own words), an ended session, a report too large, too many
//! reports, and a gateway that takes no reports at all. [`outcome`] is that
//! table, and [`ReportForm`] keeps what was typed through every refusal, so
//! "try again" never means "type it again".

use crate::i18n::{Lang, Phrase, Refusal};
use crate::textbuf::TextBuffer;

use super::{Consent, Gathered, Kind, MAX_TEXT_CHARS, Secret, Trimmed, Unsendable};

/// What the gateway's answer to a report means.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Received, under this id.
    Sent(String),
    /// Not received, and why.
    Refused(Refusal),
}

/// What an HTTP `status` with `body` means for a report. Status 0 is a
/// request that never got an answer.
#[must_use]
pub fn outcome(status: u16, body: &str) -> Outcome {
    let said = |phrase| Outcome::Refused(Refusal::Said(phrase));
    match status {
        200 | 201 => Outcome::Sent(
            serde_json::from_str::<serde_json::Value>(body)
                .ok()
                .and_then(|v| v.get("report_id").map(ToString::to_string))
                .map(|id| id.trim_matches('"').to_string())
                .unwrap_or_default(),
        ),
        400 => {
            let error = serde_json::from_str::<serde_json::Value>(body)
                .ok()
                .and_then(|v| v.get("error")?.as_str().map(str::to_string))
                .unwrap_or_else(|| body.trim().to_string());
            if error.is_empty() {
                said(Phrase::ReportFailed)
            } else {
                Outcome::Refused(Refusal::Verbatim(error))
            }
        }
        401 => said(Phrase::ReportSignInAgain),
        413 => said(Phrase::ReportTooLarge),
        429 => said(Phrase::ReportTooMany),
        502 => said(Phrase::ReportNotPassedOn),
        503 => said(Phrase::ReportsUnavailable),
        0 => said(Phrase::ReportUnreachable),
        _ => said(Phrase::ReportFailed),
    }
}

/// Where the form stands.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Status {
    /// Being written.
    #[default]
    Editing,
    /// On its way; the form waits for the answer.
    Sending,
    /// Received under this id.
    Sent(String),
    /// Refused by the gateway, or never answered. What was typed is kept.
    Failed(Refusal),
    /// Not sent, because this side found it could not be.
    Blocked(Unsendable),
}

impl Status {
    /// The status line, if the form shows one.
    #[must_use]
    pub fn text(&self, lang: Lang) -> Option<String> {
        match self {
            Self::Editing => None,
            Self::Sending => Some(Phrase::ReportSending.text(lang).to_string()),
            Self::Sent(id) => Some(Phrase::ReportSent.fill(lang, &[id])),
            Self::Failed(refusal) => Some(refusal.text(lang)),
            Self::Blocked(why) => Some(why.text(lang)),
        }
    }
}

/// The report form, without a renderer.
#[derive(Clone, Debug, Default)]
pub struct ReportForm {
    /// What kind of report.
    pub kind: Kind,
    /// What the player wrote.
    pub text: TextBuffer,
    /// Where it stands.
    pub status: Status,
    /// Whether the preview of what is sent is open.
    pub preview: bool,
    /// What the last send had to leave out to fit.
    pub trimmed: Trimmed,
}

impl ReportForm {
    /// How many characters have been written.
    #[must_use]
    pub fn chars(&self) -> usize {
        self.text.text().chars().count()
    }

    /// Whether the text is over the limit.
    #[must_use]
    pub fn over_limit(&self) -> bool {
        self.chars() > MAX_TEXT_CHARS
    }

    /// Whether Send does anything: something written, within the limit, not
    /// already on its way, and a session to send it with.
    #[must_use]
    pub fn can_send(&self, signed_in: bool) -> bool {
        signed_in
            && self.status != Status::Sending
            && !self.text.text().trim().is_empty()
            && !self.over_limit()
    }

    /// The bytes to send, having checked them; the form is then `Sending`.
    /// `None` when there is nothing to send, or when the check refused it
    /// (the form then says why).
    pub fn prepare(
        &mut self,
        gathered: &Gathered,
        consent: &Consent,
        secrets: &[Secret<'_>],
    ) -> Option<String> {
        if !self.can_send(true) {
            return None;
        }
        match gathered
            .submission(self.kind, self.text.text(), consent)
            .sealed(secrets)
        {
            Ok((json, trimmed)) => {
                self.trimmed = trimmed;
                self.status = Status::Sending;
                Some(json)
            }
            Err(why) => {
                self.status = Status::Blocked(why);
                None
            }
        }
    }

    /// The gateway answered `status` with `body`. A received report empties
    /// the form; a refused one keeps every word.
    pub fn answered(&mut self, status: u16, body: &str) {
        match outcome(status, body) {
            Outcome::Sent(id) => {
                self.text.clear();
                self.preview = false;
                self.status = Status::Sent(id);
            }
            Outcome::Refused(refusal) => self.status = Status::Failed(refusal),
        }
    }

    /// Pastes `text` over the selection, or at the caret: what the system
    /// clipboard handed the form on `Ctrl`/`Cmd`+`V`. Returns how many
    /// characters went in.
    ///
    /// Line breaks stay, because a report is prose and a pasted panic or a
    /// log excerpt is lines (`\r\n` and a lone `\r` become `\n`); every
    /// other control character is dropped, as typing drops them. And it is
    /// cut to the room the limit leaves, counting the selection it replaces
    /// as room: typing past [`MAX_TEXT_CHARS`] only greys Send, but a paste
    /// is thousands of characters at once, and a paste that overshot would
    /// leave the player to find and delete the excess by hand.
    pub fn paste(&mut self, text: &str) -> usize {
        let normal = text.replace("\r\n", "\n").replace('\r', "\n");
        let selected = self
            .text
            .selection()
            .map_or(0, |range| self.text.text()[range].chars().count());
        let room = MAX_TEXT_CHARS.saturating_sub(self.chars() - selected);
        let kept: String = normal
            .chars()
            .filter(|c| *c == '\n' || !c.is_control())
            .take(room)
            .collect();
        if kept.is_empty() {
            return 0;
        }
        self.text.replace_selection(&kept);
        self.edited();
        kept.chars().count()
    }

    /// Something was typed or ticked: a finished or refused attempt's line
    /// gives way to the form again.
    pub fn edited(&mut self) {
        if self.status != Status::Sending {
            self.status = Status::Editing;
        }
    }

    /// What would be sent, as the preview shows it: the same submission
    /// [`Self::prepare`] seals, fitted the same way, pretty-printed, with
    /// the picture's bytes written as its size.
    #[must_use]
    pub fn preview_text(&self, gathered: &Gathered, consent: &Consent) -> String {
        let submission = gathered.submission(self.kind, self.text.text(), consent);
        let Ok((fitted, _)) = submission.fitted() else {
            return String::new();
        };
        let mut value = serde_json::to_value(&fitted).unwrap_or_default();
        if let Some(shot) = value
            .get_mut("client")
            .and_then(|c| c.get_mut("screenshot"))
            .and_then(|s| s.get_mut("png_base64"))
        {
            let kb = shot.as_str().map_or(0, |s| s.len().div_ceil(1000));
            *shot = serde_json::Value::String(format!("<PNG, {kb} KB>"));
        }
        serde_json::to_string_pretty(&value).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Build, Category, Screenshot};
    use super::*;

    fn form(text: &str) -> ReportForm {
        ReportForm {
            text: TextBuffer::new(text),
            ..ReportForm::default()
        }
    }

    /// Every status the contract names reads as its own sentence, and a 400
    /// is the gateway's own words.
    #[test]
    fn each_answer_is_told_apart() {
        assert_eq!(
            outcome(201, r#"{"report_id":"r-17"}"#),
            Outcome::Sent("r-17".into())
        );
        assert_eq!(
            outcome(400, r#"{"error":"kind is not one of ..."}"#),
            Outcome::Refused(Refusal::Verbatim("kind is not one of ...".into()))
        );
        let table = [
            (401, Phrase::ReportSignInAgain),
            (413, Phrase::ReportTooLarge),
            (429, Phrase::ReportTooMany),
            (502, Phrase::ReportNotPassedOn),
            (503, Phrase::ReportsUnavailable),
            (0, Phrase::ReportUnreachable),
            (500, Phrase::ReportFailed),
        ];
        for (status, phrase) in table {
            assert_eq!(
                outcome(status, r#"{"error":"reports are not configured"}"#),
                Outcome::Refused(Refusal::Said(phrase)),
                "{status}"
            );
        }
        // Told apart means no two of them read alike, in either language.
        for lang in [Lang::En, Lang::De] {
            let mut said: Vec<&str> = table.iter().map(|(_, p)| p.text(lang)).collect();
            said.sort_unstable();
            said.dedup();
            assert_eq!(said.len(), table.len(), "{lang:?}: two statuses read alike");
        }
    }

    /// A 400 without words of the gateway's own still says something, and
    /// a 400 in plain text is quoted as it came.
    #[test]
    fn a_bad_request_is_the_gateway_s_words_or_the_general_sentence() {
        assert_eq!(
            outcome(400, ""),
            Outcome::Refused(Refusal::Said(Phrase::ReportFailed))
        );
        assert_eq!(
            outcome(400, "game_id is too long\n"),
            Outcome::Refused(Refusal::Verbatim("game_id is too long".into()))
        );
    }

    /// The form says each answer as the player reads it: the status line
    /// after a send is the outcome's sentence, and a received report says
    /// its id.
    #[test]
    fn the_status_line_after_each_answer_is_its_sentence() {
        for (status, body, want) in [
            (
                201,
                r#"{"report_id":"r-9"}"#,
                Phrase::ReportSent.fill(Lang::En, &["r-9"]),
            ),
            (400, r#"{"error":"no text"}"#, "no text".to_string()),
            (
                401,
                "",
                Phrase::ReportSignInAgain.text(Lang::En).to_string(),
            ),
            (413, "", Phrase::ReportTooLarge.text(Lang::En).to_string()),
            (429, "", Phrase::ReportTooMany.text(Lang::En).to_string()),
            (
                502,
                "",
                Phrase::ReportNotPassedOn.text(Lang::En).to_string(),
            ),
            (
                503,
                "",
                Phrase::ReportsUnavailable.text(Lang::En).to_string(),
            ),
            (0, "", Phrase::ReportUnreachable.text(Lang::En).to_string()),
        ] {
            let mut form = form("it broke");
            form.prepare(&Gathered::default(), &Consent::default(), &[])
                .expect("sendable");
            form.answered(status, body);
            assert_eq!(form.status.text(Lang::En), Some(want), "{status}");
        }
    }

    /// A refusal keeps the text for another try; a received report clears it.
    #[test]
    fn a_refusal_keeps_what_was_written() {
        let mut form = form("the stack ate my spell");
        let json = form.prepare(&Gathered::default(), &Consent::default(), &[]);
        assert!(json.is_some());
        assert_eq!(form.status, Status::Sending);
        assert!(!form.can_send(true), "one report at a time");
        form.answered(429, "");
        assert_eq!(form.text.text(), "the stack ate my spell");
        assert!(form.can_send(true));
        form.prepare(&Gathered::default(), &Consent::default(), &[]);
        form.answered(201, r#"{"report_id":"abc"}"#);
        assert!(form.text.is_empty());
        assert_eq!(form.status, Status::Sent("abc".into()));
    }

    /// Nothing is sent without a session, without text or over the limit.
    #[test]
    fn send_needs_a_session_text_and_room() {
        assert!(!form("x").can_send(false));
        assert!(!form("   ").can_send(true));
        assert!(!form(&"x".repeat(MAX_TEXT_CHARS + 1)).can_send(true));
        assert!(form(&"x".repeat(MAX_TEXT_CHARS)).can_send(true));
    }

    /// A token typed into the text blocks the send, and says so.
    #[test]
    fn a_token_in_the_text_blocks_the_send() {
        let token = "abcdefabcdefabcdefabcdef";
        let mut form = form(&format!("my token is {token}"));
        let sent = form.prepare(
            &Gathered::default(),
            &Consent::default(),
            &[Secret {
                label: "session",
                value: token,
            }],
        );
        assert!(sent.is_none());
        assert!(matches!(
            form.status,
            Status::Blocked(Unsendable::Leaked(_))
        ));
    }

    /// A paste keeps its line breaks, as one kind, and drops every other
    /// control character.
    #[test]
    fn a_paste_keeps_its_lines_and_drops_other_controls() {
        let mut form = form("");
        let went = form.paste("thread 'main' panicked\r\n  at x.rs:1\rnext\u{7}\ttab\n");
        assert_eq!(
            form.text.text(),
            "thread 'main' panicked\n  at x.rs:1\nnexttab\n"
        );
        assert_eq!(went, form.chars());
        assert_eq!(
            form.paste("\u{7}\u{1b}"),
            0,
            "nothing printable, nothing pasted"
        );
    }

    /// A paste lands at the caret, or over the selection it replaces.
    #[test]
    fn a_paste_goes_in_at_the_caret_or_over_the_selection() {
        let mut form = form("before after");
        form.text.place(7, None);
        form.paste("middle ");
        assert_eq!(form.text.text(), "before middle after");
        form.text.select_all();
        form.paste("all of it");
        assert_eq!(form.text.text(), "all of it");
    }

    /// A paste is cut to the room the limit leaves, the selection it
    /// replaces counted as room, and characters counted rather than bytes.
    #[test]
    fn a_paste_is_cut_to_the_room_the_limit_leaves() {
        let mut empty = form("");
        assert_eq!(
            empty.paste(&"ä".repeat(MAX_TEXT_CHARS + 5_000)),
            MAX_TEXT_CHARS
        );
        assert_eq!(empty.chars(), MAX_TEXT_CHARS);
        assert!(!empty.over_limit() && empty.can_send(true));
        assert_eq!(empty.paste("more"), 0, "a full box takes nothing more");
        assert_eq!(empty.chars(), MAX_TEXT_CHARS);

        let mut nearly = form_with_room(10);
        assert_eq!(nearly.paste("0123456789abc"), 10);
        assert_eq!(nearly.chars(), MAX_TEXT_CHARS);

        // Selecting five characters frees five.
        let mut full = form(&"x".repeat(MAX_TEXT_CHARS));
        full.text.place(0, Some(5));
        assert_eq!(full.paste("abcdefgh"), 5);
        assert_eq!(full.chars(), MAX_TEXT_CHARS);
        assert!(full.text.text().starts_with("abcdex"));
    }

    fn form_with_room(room: usize) -> ReportForm {
        let mut form = form(&"y".repeat(MAX_TEXT_CHARS - room));
        form.text.place(form.text.text().len(), None);
        form
    }

    /// A paste into a form whose last send was refused brings the form back
    /// to editing, as a keystroke does.
    #[test]
    fn a_paste_is_an_edit() {
        let mut form = form("x");
        form.answered(429, "");
        form.paste("y");
        assert_eq!(form.status, Status::Editing);
    }

    /// The preview is the sent body: same parts, the picture as its size.
    #[test]
    fn the_preview_shows_what_is_sent() {
        let gathered = Gathered {
            build: Build {
                version: "9.9.9".into(),
                commit: None,
            },
            screenshot: Some(Screenshot {
                width: 4,
                height: 4,
                png_base64: "A".repeat(4000),
            }),
            ..Gathered::default()
        };
        let mut consent = Consent::default();
        let form = form("hello");
        let without = form.preview_text(&gathered, &consent);
        assert!(without.contains("9.9.9") && without.contains("hello"));
        assert!(!without.contains("screenshot"));
        consent.set(Category::Screenshot, true);
        let with = form.preview_text(&gathered, &consent);
        assert!(with.contains("<PNG, 4 KB>"), "{with}");
        assert!(!with.contains("AAAA"));
    }
}
