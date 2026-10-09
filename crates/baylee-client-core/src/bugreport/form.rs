//! The report form's state, and what each answer of the gateway means.
//!
//! `POST {gateway}/reports` answers `201 {"report_id"}`, or refuses with one
//! of five statuses the player has to be told apart (#309): a bad request
//! (the gateway's own words), an ended session, a report too large, too many
//! reports, and a gateway that takes no reports at all. [`outcome`] is that
//! table, and [`ReportForm`] keeps what was typed through every refusal, so
//! "try again" never means "type it again".
//!
//! Signed in nowhere, the form sends straight to the feedback service
//! ([`Via::Direct`]); that, and any report carrying a local game's record,
//! is confirmed first, on a page listing what goes out and where
//! ([`ReportForm::parts`]).

use crate::i18n::{Lang, Phrase, Refusal};
use crate::textbuf::TextBuffer;

use super::refs::{self, Candidates, CardRef, Sigil, Trigger};
use super::{
    Category, Consent, Gathered, Kind, MAX_TEXT_CHARS, Secret, Submission, Trimmed, Unsendable,
};

/// How a report is sent: through the gateway the client is signed in to,
/// or straight to the feedback service under this device's id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Via<'a> {
    /// `POST {gateway}/reports`, with the session.
    Gateway,
    /// `POST {service}/client/reports`, under this device id.
    Direct {
        /// This device's random id ([`super::kept_device_id`]).
        device: &'a str,
    },
}

/// One thing a report sends, as the confirmation lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Part {
    /// The player's words, this many characters, and the build's version.
    Text(usize),
    /// What the words name in brackets: this many cards and players
    /// ([`super::refs`]). Only when they name any.
    Refs {
        /// Card references.
        cards: usize,
        /// Player names.
        players: usize,
    },
    /// One ticked box's contents.
    Category(Category),
    /// A local game's whole record, this many kilobytes; whether the game
    /// was over.
    Record {
        /// Its size on the wire.
        kilobytes: usize,
        /// Whether the game had ended.
        complete: bool,
    },
    /// This device's random id (a direct report only).
    Device,
}

/// What the gateway's answer to a report means.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Received, under this id.
    Sent(String),
    /// Not received, and why.
    Refused(Refusal),
}

/// What the feedback service's answer to a report sent straight to it
/// means (`direct`), or the gateway's ([`outcome`]). The service has no
/// session to end and passes nothing on, so of the gateway's sentences only
/// the two that name it differ: it may take no direct reports, and it may
/// not be there.
#[must_use]
pub fn outcome_via(status: u16, body: &str, direct: bool) -> Outcome {
    match status {
        503 if direct => Outcome::Refused(Refusal::Said(Phrase::ReportsDirectUnavailable)),
        0 if direct => Outcome::Refused(Refusal::Said(Phrase::ReportDirectUnreachable)),
        _ => outcome(status, body),
    }
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
#[expect(
    clippy::struct_excessive_bools,
    reason = "each is a different thing open or ticked, as on the client's `ReportDesk`"
)]
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
    /// Whether this report carries the local game's record: ticked for
    /// this report alone, cleared at every opening ([`Self::opened`]) and
    /// kept nowhere else.
    pub send_record: bool,
    /// Whether the confirmation is up ([`Self::needs_confirmation`]).
    pub confirming: bool,
    /// Whether the report on its way went straight to the service, for
    /// reading its answer.
    pub(crate) direct: bool,
    /// The cards the player took from the suggestions, so a name two
    /// candidates share names the one that was picked.
    pub picked: Vec<CardRef>,
    /// The sigil (its byte) whose suggestions `Esc` put away: they stay
    /// away until the reference being typed is another.
    pub dismissed: Option<usize>,
    /// The suggestion the keys are on.
    pub chosen: usize,
}

/// One suggestion: a card or a player, by its index in the candidates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Suggestion {
    /// `candidates.cards[i]`.
    Card(usize),
    /// `candidates.players[i]`.
    Player(usize),
}

/// The suggestions standing under the caret: the reference being typed and
/// the best candidates for it, never empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestions {
    /// What is being typed.
    pub trigger: Trigger,
    /// The rows, best first.
    pub rows: Vec<Suggestion>,
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

    /// The form was opened: the record's box starts unticked and no
    /// confirmation is up, whatever the last opening left. What was typed
    /// stays.
    pub fn opened(&mut self) {
        self.send_record = false;
        self.confirming = false;
    }

    /// Whether Send does anything: something written, within the limit, not
    /// already on its way, and somewhere to send it (`routed`, a gateway
    /// session or a feedback service).
    #[must_use]
    pub fn can_send(&self, routed: bool) -> bool {
        routed
            && self.status != Status::Sending
            && !self.text.text().trim().is_empty()
            && !self.over_limit()
    }

    /// What this report would send, as built from `gathered`: the text's
    /// references with it ([`refs::refs_for`]).
    fn submission(&self, gathered: &Gathered, consent: &Consent) -> Submission {
        let mut submission =
            gathered.submission_with(self.kind, self.text.text(), consent, self.send_record);
        submission.client.refs = refs::refs_for(self.text.text(), &gathered.refs, &self.picked);
        submission
    }

    /// The suggestions under the caret, if a reference is being typed, it
    /// was not put away, and anything matches. Nothing while text is
    /// selected: the caret is not at the end of a run then.
    #[must_use]
    pub fn suggestions(&self, candidates: &Candidates) -> Option<Suggestions> {
        if self.text.selection().is_some() {
            return None;
        }
        let trigger = refs::trigger(self.text.text(), self.text.cursor())?;
        if self.dismissed == Some(trigger.at.start) {
            return None;
        }
        let rows: Vec<Suggestion> = match trigger.sigil {
            Sigil::Card => refs::best(&candidates.cards, &trigger.query)
                .into_iter()
                .map(Suggestion::Card)
                .collect(),
            Sigil::Player => refs::best_players(&candidates.players, &trigger.query)
                .into_iter()
                .map(Suggestion::Player)
                .collect(),
        };
        (!rows.is_empty()).then_some(Suggestions { trigger, rows })
    }

    /// The row the keys are on, kept inside the list.
    #[must_use]
    pub fn chosen_in(&self, suggestions: &Suggestions) -> usize {
        self.chosen.min(suggestions.rows.len().saturating_sub(1))
    }

    /// `↑` (`-1`) or `↓` (`+1`) in the list, stopping at both ends.
    pub fn choose(&mut self, suggestions: &Suggestions, by: isize) {
        let last = suggestions.rows.len().saturating_sub(1);
        self.chosen = self
            .chosen_in(suggestions)
            .saturating_add_signed(by)
            .min(last);
    }

    /// Takes suggestion `row` (`None`: the one the keys are on): its name in
    /// brackets and a space over the typed run, the caret after them.
    pub fn take(&mut self, candidates: &Candidates, row: Option<usize>) {
        let Some(suggestions) = self.suggestions(candidates) else {
            return;
        };
        let row = row.unwrap_or_else(|| self.chosen_in(&suggestions));
        let Some(&suggestion) = suggestions.rows.get(row) else {
            return;
        };
        let written = match suggestion {
            Suggestion::Card(i) => {
                let card = candidates.cards[i].clone();
                let written = refs::taken(Sigil::Card, &card.name);
                self.picked.retain(|p| p.name != card.name);
                self.picked.push(card);
                written
            }
            Suggestion::Player(i) => refs::taken(Sigil::Player, &candidates.players[i].name),
        };
        let at = suggestions.trigger.at;
        self.text.place(at.end, Some(at.start));
        self.text.replace_selection(&written);
        self.chosen = 0;
        self.dismissed = None;
        self.edited();
    }

    /// `Esc` with suggestions up: they go, the typed run stays.
    pub fn dismiss(&mut self, candidates: &Candidates) {
        if let Some(suggestions) = self.suggestions(candidates) {
            self.dismissed = Some(suggestions.trigger.at.start);
        }
    }

    /// Whether Send asks first: always straight to the service, where the
    /// player has said nothing yet about where a report goes, and whenever
    /// the report carries a game's record.
    #[must_use]
    pub fn needs_confirmation(&self, gathered: &Gathered, consent: &Consent, via: Via<'_>) -> bool {
        matches!(via, Via::Direct { .. }) || gathered.record(consent, self.send_record).is_some()
    }

    /// Everything this report sends, for the confirmation to list: the
    /// words, each ticked box with something behind it, the record, and
    /// the device id when it goes straight to the service.
    #[must_use]
    pub fn parts(&self, gathered: &Gathered, consent: &Consent, via: Via<'_>) -> Vec<Part> {
        let mut parts = vec![Part::Text(self.chars())];
        if let Some(refs) = refs::refs_for(self.text.text(), &gathered.refs, &self.picked) {
            parts.push(Part::Refs {
                cards: refs.cards.len(),
                players: refs.players.len(),
            });
        }
        parts.extend(
            Category::ALL
                .into_iter()
                .filter(|c| consent.allows(*c) && gathered.has(*c))
                .map(Part::Category),
        );
        if let Some(record) = gathered.record(consent, self.send_record) {
            parts.push(Part::Record {
                kilobytes: record.kilobytes(),
                complete: record.complete,
            });
        }
        if matches!(via, Via::Direct { .. }) {
            parts.push(Part::Device);
        }
        parts
    }

    /// Send was pressed: the confirmation comes up when the report needs
    /// one, and `true` when it may go at once.
    pub fn ask_to_send(&mut self, gathered: &Gathered, consent: &Consent, via: Via<'_>) -> bool {
        if !self.can_send(true) {
            return false;
        }
        if self.needs_confirmation(gathered, consent, via) && !self.confirming {
            self.confirming = true;
            return false;
        }
        true
    }

    /// The bytes to send `via` the gateway or the service, having checked
    /// them; the form is then `Sending`. `None` when there is nothing to
    /// send, when the check refused it (the form then says why), or when
    /// the report needs a confirmation that is not up.
    pub fn prepare(
        &mut self,
        gathered: &Gathered,
        consent: &Consent,
        secrets: &[Secret<'_>],
        via: Via<'_>,
    ) -> Option<String> {
        if !self.can_send(true)
            || (self.needs_confirmation(gathered, consent, via) && !self.confirming)
        {
            return None;
        }
        self.confirming = false;
        let submission = self.submission(gathered, consent);
        let sealed = match via {
            Via::Gateway => submission.sealed(secrets),
            Via::Direct { device } => submission.sealed_direct(device, secrets),
        };
        match sealed {
            Ok((json, trimmed)) => {
                self.trimmed = trimmed;
                self.status = Status::Sending;
                self.direct = matches!(via, Via::Direct { .. });
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
        match outcome_via(status, body, self.direct) {
            Outcome::Sent(id) => {
                self.text.clear();
                self.picked.clear();
                self.preview = false;
                self.send_record = false;
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
    /// [`Self::prepare`] seals, fitted the same way and in the same shape
    /// for `via`, pretty-printed, with the picture's bytes and the record's
    /// written as their sizes.
    #[must_use]
    pub fn preview_text(&self, gathered: &Gathered, consent: &Consent, via: Via<'_>) -> String {
        let submission = self.submission(gathered, consent);
        let value = match via {
            Via::Gateway => submission
                .fitted()
                .map(|(fitted, _)| serde_json::to_value(&fitted)),
            Via::Direct { device } => submission
                .fitted_to(super::MAX_DIRECT_CLIENT_BYTES)
                .map(|(fitted, _)| serde_json::to_value(fitted.direct(device))),
        };
        let Ok(Ok(mut value)) = value else {
            return String::new();
        };
        for key in ["local_record", "record"] {
            if let Some(record) = value.get_mut(key).and_then(|r| r.get_mut("gzip_base64")) {
                let kb = record.as_str().map_or(0, |s| s.len().div_ceil(1000));
                *record = serde_json::Value::String(format!("<gzip, {kb} KB>"));
            }
        }
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
            form.prepare(&Gathered::default(), &Consent::default(), &[], Via::Gateway)
                .expect("sendable");
            form.answered(status, body);
            assert_eq!(form.status.text(Lang::En), Some(want), "{status}");
        }
    }

    /// A refusal keeps the text for another try; a received report clears it.
    #[test]
    fn a_refusal_keeps_what_was_written() {
        let mut form = form("the stack ate my spell");
        let json = form.prepare(&Gathered::default(), &Consent::default(), &[], Via::Gateway);
        assert!(json.is_some());
        assert_eq!(form.status, Status::Sending);
        assert!(!form.can_send(true), "one report at a time");
        form.answered(429, "");
        assert_eq!(form.text.text(), "the stack ate my spell");
        assert!(form.can_send(true));
        form.prepare(&Gathered::default(), &Consent::default(), &[], Via::Gateway);
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
            Via::Gateway,
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

    fn named() -> Gathered {
        use super::super::refs::{PlayerRef, RefZone};
        let card = |name: &str, zone| super::super::CardRef {
            name: name.into(),
            english: name.into(),
            card: baylee_core::ids::CardIndex::new(7),
            face: 0,
            art: None,
            print: None,
            object: None,
            zone: Some(zone),
            owner: None,
            kind: None,
        };
        Gathered {
            refs: Candidates {
                cards: vec![
                    card("Wrath of God", RefZone::Graveyard),
                    card("Wrenn and Six", RefZone::Battlefield),
                    card("Lightning Bolt", RefZone::Stack),
                ],
                players: vec![PlayerRef {
                    name: "steady 1".into(),
                    seat: Some(1),
                }],
            },
            ..Gathered::default()
        }
    }

    fn typed(text: &str) -> ReportForm {
        let mut form = form(text);
        form.text.place(text.len(), None);
        form
    }

    /// `#Wr` offers both Wr… cards, `↓` and taking writes the second in
    /// brackets with a space; `@st` writes the player.
    #[test]
    fn a_suggestion_is_taken_into_the_text() {
        let gathered = named();
        let mut form = typed("cast #Wr");
        let list = form.suggestions(&gathered.refs).expect("open");
        assert_eq!(list.rows.len(), 2);
        form.choose(&list, 1);
        form.choose(&list, 1);
        assert_eq!(form.chosen_in(&list), 1, "stops at the last row");
        form.take(&gathered.refs, None);
        assert_eq!(form.text.text(), "cast [Wrenn and Six] ");
        assert_eq!(form.text.cursor(), form.text.text().len());
        assert!(form.suggestions(&gathered.refs).is_none(), "taken, closed");

        let mut player = typed("blocked by @st");
        player.take(&gathered.refs, Some(0));
        assert_eq!(player.text.text(), "blocked by [@steady 1] ");
    }

    /// `Esc` puts the list away and leaves the run; typing on keeps it
    /// away, and the next reference opens again.
    #[test]
    fn esc_puts_the_list_away_and_keeps_the_typing() {
        let gathered = named();
        let mut form = typed("#Wr");
        form.dismiss(&gathered.refs);
        assert!(form.suggestions(&gathered.refs).is_none());
        assert_eq!(form.text.text(), "#Wr");
        form.text.insert("a");
        assert!(form.suggestions(&gathered.refs).is_none(), "still away");
        form.text.insert(" and #Li");
        assert!(form.suggestions(&gathered.refs).is_some(), "another one");
    }

    /// The confirmation names the references only when the text makes
    /// some, and the body carries them under `client.refs`.
    #[test]
    fn the_references_ride_in_the_body_and_on_the_confirmation() {
        let gathered = named();
        let consent = Consent::default();
        let plain = form("no references [here]");
        assert_eq!(
            plain.parts(&gathered, &consent, Via::Gateway),
            [Part::Text(20)]
        );
        assert!(
            !plain
                .preview_text(&gathered, &consent, Via::Gateway)
                .contains("refs")
        );
        let mut both = form("[Lightning Bolt] at [@steady 1] and [Wrath of God]");
        assert_eq!(
            both.parts(&gathered, &consent, Via::Gateway)[1],
            Part::Refs {
                cards: 2,
                players: 1
            }
        );
        let body = both
            .prepare(&gathered, &consent, &[], Via::Gateway)
            .expect("sent");
        let json: serde_json::Value = serde_json::from_str(&body).expect("json");
        let refs = &json["client"]["refs"];
        assert_eq!(refs["cards"][0]["text"], "Lightning Bolt");
        assert_eq!(refs["cards"][0]["zone"], "stack");
        assert_eq!(refs["players"][0]["seat"], 1);
        assert_eq!(refs["players"][0]["at"], serde_json::json!([20, 31]));
    }

    /// A report naming nothing is the body it was before references: the
    /// same bytes.
    #[test]
    fn a_report_naming_nothing_is_the_body_it_was() {
        let consent = Consent::default();
        let mut without = form("it broke [badly] #3");
        let mut with = form("it broke [badly] #3");
        let before = without
            .prepare(&Gathered::default(), &consent, &[], Via::Gateway)
            .expect("sent");
        let after = with
            .prepare(&named(), &consent, &[], Via::Gateway)
            .expect("sent");
        assert_eq!(before, after);
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
        let without = form.preview_text(&gathered, &consent, Via::Gateway);
        assert!(without.contains("9.9.9") && without.contains("hello"));
        assert!(!without.contains("screenshot"));
        consent.set(Category::Screenshot, true);
        let with = form.preview_text(&gathered, &consent, Via::Gateway);
        assert!(with.contains("<PNG, 4 KB>"), "{with}");
        assert!(!with.contains("AAAA"));
    }

    fn local() -> Gathered {
        Gathered {
            local_record: super::super::LocalRecord::pack(
                br#"{"kind":"header","record":1,"build":"0.1.0","preset":{},"hash":"00"}
"#,
            ),
            ..Gathered::default()
        }
    }

    const DEVICE: &str = "0123456789abcdef0123456789abcdef";

    /// The record's yes is this report's alone: every opening clears it,
    /// and so does a received report.
    #[test]
    fn the_records_box_is_unticked_at_every_opening() {
        let mut form = form("it broke");
        form.send_record = true;
        form.confirming = true;
        form.opened();
        assert!(!form.send_record && !form.confirming);
        assert_eq!(form.text.text(), "it broke", "the words stay");

        form.send_record = true;
        assert!(!form.ask_to_send(&local(), &Consent::default(), Via::Gateway));
        assert!(form.confirming);
        form.prepare(&local(), &Consent::default(), &[], Via::Gateway)
            .expect("confirmed");
        form.answered(201, r#"{"report_id":"r-1"}"#);
        assert!(
            !form.send_record,
            "a sent record is not offered as sent again"
        );
    }

    /// A report carrying a record, and any report straight to the service,
    /// is confirmed first; a gateway report of words alone goes at once.
    /// Nothing is prepared while a needed confirmation is not up.
    #[test]
    fn a_record_or_a_direct_report_is_confirmed_first() {
        let consent = Consent::default();
        let direct = Via::Direct { device: DEVICE };
        let mut words = form("words alone");
        assert!(words.ask_to_send(&local(), &consent, Via::Gateway));
        assert!(!words.confirming);

        for (record, via) in [(true, Via::Gateway), (false, direct), (true, direct)] {
            let mut form = form("x");
            form.send_record = record;
            assert!(form.needs_confirmation(&local(), &consent, via));
            assert_eq!(
                form.prepare(&local(), &consent, &[], via),
                None,
                "unconfirmed"
            );
            assert_eq!(form.status, Status::Editing);
            assert!(!form.ask_to_send(&local(), &consent, via));
            assert!(form.confirming);
            assert!(form.ask_to_send(&local(), &consent, via), "confirmed");
            let body = form.prepare(&local(), &consent, &[], via).expect("sent");
            assert!(!form.confirming);
            assert_eq!(body.contains("gzip_base64"), record, "{via:?}");
            assert_eq!(body.contains(DEVICE), via != Via::Gateway);
        }
        let never = Consent {
            record: super::super::RecordConsent::Never,
            ..Consent::default()
        };
        let mut form = form("x");
        form.send_record = true;
        assert!(!form.needs_confirmation(&local(), &never, Via::Gateway));
        let body = form
            .prepare(&local(), &never, &[], Via::Gateway)
            .expect("words alone go at once");
        assert!(!body.contains("gzip_base64"), "never means never");
    }

    /// The confirmation lists exactly what goes: the words, each ticked box
    /// with something behind it, the record when ticked, the device id
    /// when straight to the service.
    #[test]
    fn the_confirmation_lists_what_goes() {
        let mut gathered = local();
        gathered.system = Some(super::super::System::default());
        let mut consent = Consent::default();
        consent.set(Category::System, true);
        consent.set(Category::Screenshot, true);
        let mut form = form("four");
        assert_eq!(
            form.parts(&gathered, &consent, Via::Gateway),
            [Part::Text(4), Part::Category(Category::System)]
        );
        form.send_record = true;
        let parts = form.parts(&gathered, &consent, Via::Direct { device: DEVICE });
        assert!(matches!(
            parts.as_slice(),
            [
                Part::Text(4),
                Part::Category(Category::System),
                Part::Record {
                    complete: false,
                    ..
                },
                Part::Device
            ]
        ));
    }

    /// The service's answers read as the service's: its own sentence where
    /// the gateway's would name a gateway, the gateway's elsewhere.
    #[test]
    fn a_direct_answer_names_the_service() {
        assert_eq!(
            outcome_via(503, "", true),
            Outcome::Refused(Refusal::Said(Phrase::ReportsDirectUnavailable))
        );
        assert_eq!(
            outcome_via(0, "", true),
            Outcome::Refused(Refusal::Said(Phrase::ReportDirectUnreachable))
        );
        for status in [201, 400, 413, 429, 500] {
            assert_eq!(
                outcome_via(status, "", true),
                outcome(status, ""),
                "{status}"
            );
        }
        assert_eq!(outcome_via(503, "", false), outcome(503, ""));

        let mut form = form("x");
        form.ask_to_send(
            &Gathered::default(),
            &Consent::default(),
            Via::Direct { device: DEVICE },
        );
        form.prepare(
            &Gathered::default(),
            &Consent::default(),
            &[],
            Via::Direct { device: DEVICE },
        )
        .expect("sent");
        form.answered(503, "");
        assert_eq!(
            form.status,
            Status::Failed(Refusal::Said(Phrase::ReportsDirectUnavailable))
        );
    }

    /// The preview shows the body as it goes, by either road, the record as
    /// its size.
    #[test]
    fn the_preview_shows_the_record_as_its_size_by_either_road() {
        let mut form = form("x");
        form.send_record = true;
        let consent = Consent::default();
        let gateway = form.preview_text(&local(), &consent, Via::Gateway);
        assert!(gateway.contains("\"local_record\"") && gateway.contains("<gzip, "));
        assert!(!gateway.contains(DEVICE));
        let direct = form.preview_text(&local(), &consent, Via::Direct { device: DEVICE });
        assert!(direct.contains("\"record\"") && direct.contains("<gzip, "));
        assert!(direct.contains(DEVICE) && !direct.contains("local_record"));
    }
}
