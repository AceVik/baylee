//! What a bug report carries, and what it must never carry.
//!
//! A player who has just watched the game do the wrong thing is holding the
//! only copy of the evidence, and the window that says "what went wrong?" is
//! the one chance to collect it. So this can reach for a great deal — the
//! whole board as the seat was shown it, the question the engine was asking,
//! what the client had selected and armed, the seat's own game log, the
//! settings, a picture of the window — and the free text beside it, because
//! the one thing no dump contains is what the player *expected* to happen.
//!
//! What it reaches for is the player's choice (#309, #314): every part past
//! the build is one [`Category`], one box in the form, and [`Consent`], kept
//! per device, says which are ticked. [`Gathered::report`] copies a part in
//! only when its box is, and the form's preview shows the very bytes
//! [`Submission::sealed`] hands the sender.
//!
//! A report about a game this device hosted itself may carry that game's
//! record ([`LocalRecord`]), which is a different kind of thing: the whole
//! game, every seat's hidden cards included. It is no [`Category`]; it is
//! attached only when the player ticks it for the one report
//! ([`Gathered::submission`]'s `send_record`), and never under a game id.
//!
//! Signed in, a report goes to that gateway; signed in nowhere, straight to
//! the feedback service the client knows, or nowhere ([`route`]).
//!
//! # Why that is safe
//!
//! The alarming half of "send me everything" is that a client is a program a
//! player runs. Three rules, and they are different rules with different
//! reasons.
//!
//! **An opponent's information cannot be in here, because the client never
//! had it.** `baylee-view` has no field to leak through: a library is a
//! count, another seat's hand is a count, a face-down permanent's `card` is
//! `None` for anyone not entitled to look. That is the platform's own rule
//! (`docs/protocol.md`, "hidden information is unrepresentable, not
//! omitted"), and it is what makes a report of the *whole* `PlayerView`
//! safe to attach without reading it first. Nothing has to be filtered out,
//! because nothing was ever put in.
//!
//! **Other players' names are replaced where they enter.** A name is in one
//! place only, the roster in `GameStatic`, and the log writes it into lines.
//! [`seat_log`] writes the log against a roster whose names are already
//! "Player A", "Player B", so no line is ever written with the real one.
//! The roster itself is not attached, and the view names nobody.
//!
//! **The reporter's own secrets are kept out by an allow-list, never by a
//! filter.** [`BugReport`] is a named struct assembled one field at a time;
//! there is deliberately no "and the rest of the client state" field, and
//! the `dev-control` `/state` dump — which is the tempting shortcut — is
//! exactly the wrong shape, because it is a debugging surface that grows,
//! and the day it grows a field holding a token, a dump ships the token and
//! a struct does not. A seat token, a session token and a gateway URL
//! carrying either are not fields here, and neither is the account's
//! address: a gateway learns *who* from the request it authenticated, so
//! identity never has to ride in the payload at all.
//!
//! And because "we were careful" is not a property anything can check,
//! [`seal`] makes it one. The sender hands over the secrets it is holding
//! and gets the bytes back only if none of them appears in the report. It
//! fails closed, at run time, in the shipped build — so a field added later
//! that happens to carry a token is a refused report rather than a leaked
//! one.

use serde::{Deserialize, Serialize};

use baylee_engine::choice::Pending;
use baylee_view::PlayerView;

use crate::i18n::Phrase;

mod base64;
mod consent;
mod crash;
mod form;
mod localrecord;
pub mod refs;
mod route;
mod seatlog;

pub use base64::encode as base64_encode;
pub use consent::{Category, Consent, CrashConsent, RecordConsent};
pub use crash::{
    BACKTRACE_CHARS, CrashFile, CrashRecord, CrashStep, bounded_backtrace, crash_step,
    crash_submission, scrub_home,
};
pub use form::{
    Outcome, Part, ReportForm, Status, Suggestion, Suggestions, Via, outcome, outcome_via,
};
pub use localrecord::{
    KEEP_RECORD_BYTES, KEEP_RECORDS, LocalRecord, MAX_RECORD_BYTES, ReadBack, gzip,
    is_record_file_name, read_back, record_file_name, retention,
};
pub use refs::{Candidates, CardRef, PlayerRef, Refs};
pub use route::{
    DEVICE_ID_CHARS, DIRECT_PATH, DirectSubmission, MAX_DIRECT_CLIENT_BYTES, Route, device_id,
    feedback_service, is_device_id, kept_device_id, route,
};
pub use seatlog::{LogRow, RosterSeat, SeatLog, seat_log};

/// The shortest string [`seal`] will search for.
///
/// A secret short enough to occur by accident would make every report a
/// refused one: a two-character token appears in the first card name that
/// contains it. Anything this platform calls a token is 32 hex characters or
/// a base64url blob, so the bound costs nothing real — and a "secret" below
/// it is not one, whatever it is stored in.
pub const SHORTEST_SECRET: usize = 12;

/// The longest text a report may carry, in characters, as the gateway
/// counts them (`POST /reports`, the `text` field).
pub const MAX_TEXT_CHARS: usize = 20_000;

/// The most the `client` object may weigh once serialised, in bytes.
///
/// The gateway refuses more than 2 MB; this is the smaller reading of that
/// (10^6, not 2^20), so a report this side lets through is never one the
/// gateway turns away for its size.
pub const MAX_CLIENT_BYTES: usize = 2_000_000;

/// One thing the sender is holding that must not appear in a report.
///
/// The `label` is what a refusal is allowed to say. The value never is: an
/// error message naming the string it found would be a way to read a token
/// out of a log.
#[derive(Clone, Copy, Debug)]
pub struct Secret<'a> {
    /// What it is, for a refusal to name.
    pub label: &'a str,
    /// The bytes that must not be in the report.
    pub value: &'a str,
}

/// Every token the client is holding, wherever the shell keeps it, for
/// [`seal`] to look for.
///
/// Gathered afresh for each send, from every place a token lives rather than
/// from the screen that happens to be up: the seat token is the lobby's only
/// between a join and the table opening, and after that it is the table's
/// host's, which is exactly when a report about a game gets written. A
/// keyring that missed one screen's token would pass the one report most
/// likely to hold it.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Keyring {
    /// Account sessions: the lobby's, and every copy of it (preferences,
    /// card text) that could have gone stale behind a sign-out.
    pub sessions: Vec<String>,
    /// Seat tokens: the lobby's handover and the table's ticket.
    pub seats: Vec<String>,
    /// Guests this device keeps, one per gateway.
    pub guests: Vec<String>,
}

impl std::fmt::Debug for Keyring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Keyring")
            .field("sessions", &self.sessions.len())
            .field("seats", &self.seats.len())
            .field("guests", &self.guests.len())
            .finish()
    }
}

impl Keyring {
    /// The label a refusal names a session by.
    pub const SESSION: &'static str = "session token";
    /// The label a refusal names a seat token by.
    pub const SEAT: &'static str = "seat token";
    /// The label a refusal names a kept guest by.
    pub const GUEST: &'static str = "guest token";

    /// Every token, labelled, for [`seal`].
    #[must_use]
    pub fn secrets(&self) -> Vec<Secret<'_>> {
        [
            (Self::SESSION, &self.sessions),
            (Self::SEAT, &self.seats),
            (Self::GUEST, &self.guests),
        ]
        .into_iter()
        .flat_map(|(label, values)| {
            values.iter().map(move |value| Secret {
                label,
                value: value.as_str(),
            })
        })
        .collect()
    }
}

/// A report that was refused because it contained something it must not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Leaked {
    /// Which secret, by its label. Never the secret itself.
    pub label: String,
}

impl std::fmt::Display for Leaked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the report contains the {}; refusing to send it",
            self.label
        )
    }
}

impl std::error::Error for Leaked {}

/// What kind of report this is: the `kind` field of `POST /reports`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Something did the wrong thing.
    #[default]
    Bug,
    /// Something could be better.
    Improvement,
    /// Anything the player wants to say about the game.
    Feedback,
    /// The client stopped. Sent by the client itself, never picked in the
    /// form: see [`crash_submission`].
    Crash,
    /// None of the above.
    Other,
}

impl Kind {
    /// The kinds a player picks between, in the order the form offers them.
    pub const OFFERED: [Self; 4] = [Self::Bug, Self::Improvement, Self::Feedback, Self::Other];

    /// How the form names it.
    #[must_use]
    pub fn phrase(self) -> Phrase {
        match self {
            Self::Bug => Phrase::ReportKindBug,
            Self::Improvement => Phrase::ReportKindImprovement,
            Self::Feedback => Phrase::ReportKindFeedback,
            Self::Crash => Phrase::ReportKindCrash,
            Self::Other => Phrase::ReportKindOther,
        }
    }
}

/// Which build this was. Always sent: a report nobody can match to a build
/// cannot be read, and neither field says anything about a person.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Build {
    /// The crate version.
    pub version: String,
    /// The commit, when the build was told one.
    pub commit: Option<String>,
}

/// What the client was running on ([`Category::System`]).
///
/// Every field here is about the program and the machine, not about the
/// person. A GPU adapter string and a window size are what separate "the
/// client is wrong" from "this machine draws it differently", and neither
/// says who is at the keyboard.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct System {
    /// `target_os/target_arch`.
    pub platform: String,
    /// Logical CPUs, where the platform says.
    pub cpus: Option<usize>,
    /// What the renderer picked, verbatim.
    pub adapter: Option<String>,
    /// The graphics backend (Metal, Vulkan, WebGPU, …).
    pub backend: Option<String>,
    /// Logical window size.
    pub window: (u32, u32),
    /// Device pixels per logical pixel.
    pub scale: f32,
    /// The language the interface was speaking.
    pub lang: String,
}

/// Where in a game the report was written.
///
/// `seat` is a **number**, which is the whole point of this struct existing
/// rather than the fields being loose: the thing next to a seat number in
/// every other part of the client is the seat *token*, and the two must
/// never be typed into the same place by mistake. There is no field for it
/// here and there is no reason for one — the operator resolves a game and a
/// seat number against their own records.
#[derive(Clone, Debug, Serialize)]
pub struct Table {
    /// Which seat the reporter was sitting in.
    pub seat: u8,
    /// The view the report was written against, so the operator can line it
    /// up with the engine's own record of the same moment.
    pub seq: u64,
    /// Turn number, phase and step, in the interface's own spelling.
    pub when: String,
}

/// What the client was in the middle of.
///
/// The three states `dev-control` reports for the same reason a player
/// cannot see them: an action built and never sent, a run of lands being
/// tapped, an ability chooser holding the keyboard. All three look exactly
/// like "the key did nothing", which is how most of these reports start.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Holding {
    /// Objects the player had selected towards the current answer.
    pub selected: usize,
    /// The armed deed, rendered — a `Play`, an `Ability` or a mana `Run`.
    pub armed: Option<String>,
    /// Whether the client was tapping lands on the player's behalf.
    pub mana_run: bool,
    /// Actions built and not yet sent.
    pub outbox: usize,
    /// The engine's refusal of the last action, verbatim.
    pub last_error: Option<String>,
}

/// The table as the reporting seat was shown it ([`Category::Game`]).
#[derive(Clone, Debug, Serialize)]
pub struct Game {
    /// Where in the game.
    pub table: Table,
    /// The whole board as this seat was shown it.
    ///
    /// Attached whole, and that is the point: what the reporter can see is
    /// exactly what this carries, so there is nothing here to redact. It
    /// names no player either: names live in the roster, which is not here.
    pub view: PlayerView,
    /// The question the engine was asking.
    pub pending: Option<Pending>,
    /// What the client was in the middle of.
    pub holding: Holding,
}

/// The settings that shape what the player saw ([`Category::Settings`]).
///
/// A summary written field by field and never the settings file itself:
/// that file holds the gateway list, the last username and a kept guest's
/// token, none of which belongs in a report.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Settings {
    /// The interface language.
    pub lang: String,
    /// How large the card preview is drawn.
    pub preview_scale: f32,
    /// Whether the constructed text face stands in for card art.
    pub prefer_text_view: bool,
    /// How the zone browser lays cards out.
    pub zone_view: String,
    /// The music's level.
    pub music: String,
    /// How many gateways this device has saved: a count, never an address.
    pub saved_gateways: usize,
    /// The account's preferences (key bindings, standing answers,
    /// automation), as the client keeps them. They name abilities and keys,
    /// never people.
    pub preferences: Option<serde_json::Value>,
}

/// A picture of the window ([`Category::Screenshot`]).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Screenshot {
    /// Pixels across.
    pub width: u32,
    /// Pixels down.
    pub height: u32,
    /// The PNG, base64 (RFC 4648, padded).
    pub png_base64: String,
}

impl Screenshot {
    /// The picture's size on the wire, in kilobytes, for the form to say.
    #[must_use]
    pub fn kilobytes(&self) -> usize {
        self.png_base64.len().div_ceil(1000)
    }
}

/// Everything a report carries in its `client` object.
///
/// Assembled field by field on purpose — see the module header. A field
/// added here is a decision about what leaves a player's machine, which is
/// why there is no catch-all and why [`seal`] exists. Every optional part but
/// `crash` is one [`Category`] and is `None` unless the player ticked it.
#[derive(Clone, Debug, Default, Serialize)]
pub struct BugReport {
    /// Which build. Always there.
    pub build: Build,
    /// What it ran on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<System>,
    /// The table as this seat saw it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game: Option<Game>,
    /// This seat's own log, other players' names replaced.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log: Option<SeatLog>,
    /// The settings that shape what the player saw.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Settings>,
    /// A picture of the window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<Screenshot>,
    /// What stopped the client, in a crash report.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crash: Option<CrashRecord>,
    /// The cards and players the text names in brackets ([`refs`]): derived
    /// from the text, so as consented as the text, and absent when it names
    /// none. Positions, registry indexes, printings and seat numbers; never
    /// an account.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refs: Option<Refs>,
}

/// Everything the client could attach, before the player has chosen.
///
/// The form keeps one of these and builds a [`Submission`] from it and the
/// player's [`Consent`] each time it is shown or sent, so what the preview
/// shows and what goes out are one computation.
#[derive(Clone, Debug, Default)]
pub struct Gathered {
    /// Which build.
    pub build: Build,
    /// The game's id at the gateway, when there is a networked game.
    pub game_id: Option<String>,
    /// What it runs on.
    pub system: Option<System>,
    /// The table, when there is one.
    pub game: Option<Game>,
    /// The seat's log, already written by [`seat_log`].
    pub log: Option<SeatLog>,
    /// The settings summary.
    pub settings: Option<Settings>,
    /// The picture, once it has been taken.
    pub screenshot: Option<Screenshot>,
    /// The record of the game this device hosted, packed once.
    pub local_record: Option<LocalRecord>,
    /// What the text may name ([`refs`]): the seat's own view at a table,
    /// the compiled pool elsewhere. Kept here, never sent as it stands.
    pub refs: Candidates,
}

impl Gathered {
    /// Whether there is anything to attach under `category`.
    ///
    /// A box with nothing behind it is still shown and can still be ticked:
    /// the ticks are the player's standing answer on this device, kept for
    /// the next report (which may be written at a table, with a log and a
    /// picture), and a box that refused a click now and took one later would
    /// be a box that seemed broken. What it cannot do is put anything in the
    /// report: [`Self::report`] asks this, so a ticked box over nothing sends
    /// no key at all, not an empty one.
    #[must_use]
    pub fn has(&self, category: Category) -> bool {
        match category {
            Category::System => self.system.is_some(),
            Category::Game => self.game.is_some(),
            Category::Log => self.log.as_ref().is_some_and(|log| !log.lines.is_empty()),
            Category::Settings => self.settings.is_some(),
            Category::Screenshot => self.screenshot.is_some(),
        }
    }

    /// The report, holding exactly what `consent` allows.
    ///
    /// An allow-list in the literal sense: each part is copied in only when
    /// its box is ticked *and* [`Self::has`] something under it, and nothing
    /// is built first and stripped after.
    #[must_use]
    pub fn report(&self, consent: &Consent) -> BugReport {
        let take = |category: Category| consent.allows(category) && self.has(category);
        BugReport {
            build: self.build.clone(),
            system: self.system.clone().filter(|_| take(Category::System)),
            game: self.game.clone().filter(|_| take(Category::Game)),
            log: self.log.clone().filter(|_| take(Category::Log)),
            settings: self.settings.clone().filter(|_| take(Category::Settings)),
            screenshot: self
                .screenshot
                .clone()
                .filter(|_| take(Category::Screenshot)),
            crash: None,
            refs: None,
        }
    }

    /// The record this report would carry: the local game's, when the
    /// device has not said "never" ([`RecordConsent`]) and the player
    /// ticked it for this report (`send_record`). Never a networked game's:
    /// its gateway keeps that one itself.
    #[must_use]
    pub fn record(&self, consent: &Consent, send_record: bool) -> Option<&LocalRecord> {
        self.local_record
            .as_ref()
            .filter(|_| send_record && consent.record == RecordConsent::Ask)
            .filter(|_| self.game_id.is_none())
    }

    /// Whether the form offers the record's box at all.
    #[must_use]
    pub fn offers_record(&self, consent: &Consent) -> bool {
        self.record(consent, true).is_some()
    }

    /// The whole body of a report of `kind` saying `text`, without a
    /// record: the safe side, for every caller that is not the form.
    #[must_use]
    pub fn submission(&self, kind: Kind, text: &str, consent: &Consent) -> Submission {
        self.submission_with(kind, text, consent, false)
    }

    /// The whole body of a report of `kind` saying `text`, carrying the
    /// local game's record when `send_record` (see [`Self::record`]).
    #[must_use]
    pub fn submission_with(
        &self,
        kind: Kind,
        text: &str,
        consent: &Consent,
        send_record: bool,
    ) -> Submission {
        let local_record = self.record(consent, send_record).cloned();
        Submission {
            kind,
            text: text.to_string(),
            // A report is about a game of the gateway's or a local one: a
            // record the client wrote never rides under a game id.
            game_id: self.game_id.clone().filter(|_| local_record.is_none()),
            client: self.report(consent),
            local_record,
        }
    }
}

/// The body of `POST {gateway}/reports`, field for field.
#[derive(Clone, Debug, Serialize)]
pub struct Submission {
    /// What kind of report.
    pub kind: Kind,
    /// What the player wrote.
    pub text: String,
    /// The game it was written during, when it was a networked one. The
    /// gateway attaches that game's full record itself.
    pub game_id: Option<String>,
    /// Everything else, as far as the player allowed it.
    pub client: BugReport,
    /// The record of a game the client hosted itself, when the player
    /// ticked it for this report: unverified, and marked so by the gateway.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_record: Option<LocalRecord>,
}

/// A submission that cannot be sent as it stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unsendable {
    /// One of the sender's secrets is in it.
    Leaked(Leaked),
    /// The text is longer than [`MAX_TEXT_CHARS`].
    TextTooLong,
    /// The `client` object is over [`MAX_CLIENT_BYTES`] even with the
    /// screenshot and the log left out.
    TooLarge,
}

impl Unsendable {
    /// What the form says about it.
    #[must_use]
    pub fn text(&self, lang: crate::i18n::Lang) -> String {
        match self {
            Self::Leaked(leaked) => Phrase::ReportLeaked.fill(lang, &[&leaked.label]),
            Self::TextTooLong => {
                Phrase::ReportTextTooLong.fill(lang, &[&MAX_TEXT_CHARS.to_string()])
            }
            Self::TooLarge => Phrase::ReportTooLarge.text(lang).to_string(),
        }
    }
}

/// What [`Submission::fitted`] had to leave out to stay under the limit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Trimmed {
    /// The screenshot was dropped.
    pub screenshot: bool,
    /// The log was cut to its last this many lines.
    pub log_lines: Option<usize>,
    /// The game's record was too large to ride and was left out.
    pub record: bool,
}

impl Submission {
    /// This submission cut to [`MAX_CLIENT_BYTES`], and what was cut.
    ///
    /// # Errors
    ///
    /// [`Unsendable::TooLarge`] when even that is not enough.
    pub fn fitted(self) -> Result<(Self, Trimmed), Unsendable> {
        self.fitted_to(MAX_CLIENT_BYTES)
    }

    /// This submission with its `client` object cut to `limit` bytes, and
    /// what was cut.
    ///
    /// The screenshot goes first, because it is by far the heaviest part and
    /// the one a report can best do without; then the log loses its oldest
    /// half, again and again, because the lines nearest the report are the
    /// ones it is about. A record over [`MAX_RECORD_BYTES`] is left out
    /// rather than refusing the report: the words still go.
    ///
    /// # Errors
    ///
    /// [`Unsendable::TooLarge`] when even that is not enough.
    pub fn fitted_to(mut self, limit: usize) -> Result<(Self, Trimmed), Unsendable> {
        fn weight(report: &BugReport) -> usize {
            serde_json::to_vec(report).map_or(usize::MAX, |bytes| bytes.len())
        }
        let mut trimmed = Trimmed::default();
        if self.local_record.as_ref().is_some_and(|r| !r.fits()) {
            self.local_record = None;
            trimmed.record = true;
        }
        if weight(&self.client) <= limit {
            return Ok((self, trimmed));
        }
        trimmed.screenshot = self.client.screenshot.take().is_some();
        while weight(&self.client) > limit {
            let Some(log) = self.client.log.as_mut() else {
                return Err(Unsendable::TooLarge);
            };
            if log.lines.is_empty() {
                self.client.log = None;
                trimmed.log_lines = Some(0);
                continue;
            }
            let keep = log.lines.len() / 2;
            log.lines.drain(..log.lines.len() - keep);
            trimmed.log_lines = Some(keep);
        }
        Ok((self, trimmed))
    }

    /// The bytes to send: checked, fitted and sealed, in that order, so that
    /// [`seal`] is the last thing to look at them.
    ///
    /// # Errors
    ///
    /// See [`Unsendable`].
    pub fn sealed(self, secrets: &[Secret<'_>]) -> Result<(String, Trimmed), Unsendable> {
        if self.text.chars().count() > MAX_TEXT_CHARS {
            return Err(Unsendable::TextTooLong);
        }
        let (fitted, trimmed) = self.fitted()?;
        seal_record(fitted.local_record.as_ref(), secrets)?;
        let json = seal(&fitted, secrets).map_err(Unsendable::Leaked)?;
        Ok((json, trimmed))
    }

    /// The body to send straight to the feedback service under `device`
    /// ([`DirectSubmission`]): checked, fitted to the service's smaller
    /// bound ([`MAX_DIRECT_CLIENT_BYTES`]) and sealed, as [`Self::sealed`].
    ///
    /// # Errors
    ///
    /// See [`Unsendable`].
    pub fn sealed_direct(
        self,
        device: &str,
        secrets: &[Secret<'_>],
    ) -> Result<(String, Trimmed), Unsendable> {
        if self.text.chars().count() > MAX_TEXT_CHARS {
            return Err(Unsendable::TextTooLong);
        }
        let (fitted, trimmed) = self.fitted_to(MAX_DIRECT_CLIENT_BYTES)?;
        let direct = fitted.direct(device);
        seal_record(direct.record.as_ref(), secrets)?;
        let json = seal(&direct, secrets).map_err(Unsendable::Leaked)?;
        Ok((json, trimmed))
    }

    /// This submission in the shape the service takes straight from a
    /// client: the same text, parts and record; the build beside them; the
    /// device's id where a gateway would have named the account; no game
    /// id, since no gateway is there to know one.
    #[must_use]
    pub fn direct(self, device: &str) -> DirectSubmission {
        DirectSubmission {
            kind: self.kind,
            text: self.text,
            build: self.client.build.clone(),
            device: device.to_string(),
            client: self.client,
            record: self.local_record,
        }
    }
}

/// [`seal`] for a record, which the serialised report carries as base64 of
/// a gzip that no search for a token could see into: its own lines are
/// searched instead.
fn seal_record(record: Option<&LocalRecord>, secrets: &[Secret<'_>]) -> Result<(), Unsendable> {
    let Some(record) = record else {
        return Ok(());
    };
    for secret in secrets {
        if secret.value.len() >= SHORTEST_SECRET && record.plain().contains(secret.value) {
            return Err(Unsendable::Leaked(Leaked {
                label: secret.label.to_string(),
            }));
        }
    }
    Ok(())
}

/// The report as bytes, or a refusal because one of `secrets` is in it.
///
/// This is the only way to get a report out of this module, and the only
/// reason it is: "the struct has no field for a token" is a claim about
/// today's struct, and a report is assembled from half a dozen places that
/// each grow. Serialising first and searching the bytes afterwards is the
/// one check that stays true when someone adds a field — it does not care
/// *where* the secret got in, only that it is not going out.
///
/// A secret shorter than [`SHORTEST_SECRET`] is not searched for: see that
/// constant. An empty one is skipped for the same reason.
///
/// # Errors
///
/// [`Leaked`] when one of `secrets` appears in the serialised report, naming
/// the secret's label and never its value.
///
/// # Panics
///
/// Never for a [`Submission`] or a [`BugReport`]: every field serialises,
/// and the view and the pending question are the wire types the client
/// already receives as JSON.
pub fn seal<T: Serialize>(report: &T, secrets: &[Secret<'_>]) -> Result<String, Leaked> {
    let json = serde_json::to_string(report).expect("a bug report serialises");
    for secret in secrets {
        if secret.value.len() < SHORTEST_SECRET {
            continue;
        }
        if json.contains(secret.value) {
            return Err(Leaked {
                label: secret.label.to_string(),
            });
        }
    }
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ViewBuilder;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef";

    fn build() -> Build {
        Build {
            version: "0.1.0".into(),
            commit: Some("deadbeef".into()),
        }
    }

    fn system() -> System {
        System {
            platform: "macos/aarch64".into(),
            cpus: Some(10),
            adapter: Some("Apple M1 Max".into()),
            backend: Some("Metal".into()),
            window: (1728, 1052),
            scale: 2.0,
            lang: "de".into(),
        }
    }

    fn gathered() -> Gathered {
        Gathered {
            build: build(),
            game_id: Some("g-42".into()),
            system: Some(system()),
            game: Some(Game {
                table: Table {
                    seat: 0,
                    seq: 7,
                    when: "turn 3, main 1".into(),
                },
                view: ViewBuilder::new(2).build(),
                pending: None,
                holding: Holding::default(),
            }),
            log: Some(SeatLog {
                seat: 0,
                roster: Vec::new(),
                lines: vec![LogRow {
                    turn: 1,
                    at: 0,
                    text: "Your turn".into(),
                }],
            }),
            settings: Some(Settings {
                lang: "de".into(),
                music: "soft".into(),
                ..Settings::default()
            }),
            screenshot: Some(Screenshot {
                width: 2,
                height: 1,
                png_base64: "iVBORw0KGgo=".into(),
            }),
            local_record: None,
            refs: Candidates::default(),
        }
    }

    fn told() -> &'static str {
        "Der Command Tower wurde nicht getappt; Harabaz Druid haette gehen sollen"
    }

    fn token() -> [Secret<'static>; 1] {
        [Secret {
            label: "seat token",
            value: TOKEN,
        }]
    }

    fn json_of(submission: Submission) -> serde_json::Value {
        let (json, _) = submission.sealed(&token()).expect("nothing to find");
        serde_json::from_str(&json).expect("sealed bytes are JSON")
    }

    /// The ordinary case: a report with no secret in it comes back as bytes,
    /// shaped as `POST /reports` takes it.
    #[test]
    fn a_clean_report_is_handed_over_in_the_contract_s_shape() {
        let all = Consent::everything();
        let body = json_of(gathered().submission(Kind::Improvement, told(), &all));
        assert_eq!(body["kind"], "improvement");
        assert_eq!(body["text"], told());
        assert_eq!(body["game_id"], "g-42");
        let client = body["client"].as_object().expect("client is an object");
        for part in ["build", "system", "game", "log", "settings", "screenshot"] {
            assert!(
                client.contains_key(part),
                "{part} was ticked and is missing"
            );
        }
    }

    /// Nothing ticked: the build and nothing else. This is the consent gate,
    /// read off the bytes that would be sent rather than off the struct.
    #[test]
    fn with_nothing_ticked_only_the_build_goes() {
        let body = json_of(gathered().submission(Kind::Bug, told(), &Consent::default()));
        let client = body["client"].as_object().expect("client is an object");
        assert_eq!(client.keys().collect::<Vec<_>>(), ["build"]);
        assert!(
            !body.to_string().contains("Apple M1 Max"),
            "the adapter is a system detail, and system was not ticked"
        );
    }

    /// Each box lets through its own part and no other.
    #[test]
    fn each_box_lets_through_its_own_part_and_no_other() {
        let key = |category| match category {
            Category::System => "system",
            Category::Game => "game",
            Category::Log => "log",
            Category::Settings => "settings",
            Category::Screenshot => "screenshot",
        };
        for category in Category::ALL {
            let mut consent = Consent::default();
            consent.set(category, true);
            let body = json_of(gathered().submission(Kind::Bug, "x", &consent));
            let client = body["client"].as_object().expect("client is an object");
            let mut keys: Vec<_> = client.keys().map(String::as_str).collect();
            keys.sort_unstable();
            let mut want = vec!["build", key(category)];
            want.sort_unstable();
            assert_eq!(keys, want, "{category:?}");
        }
    }

    /// A box ticked over nothing sends nothing: no key, not an empty one.
    /// Here every box is ticked, the lobby had no table (no game, no log),
    /// the log a table left behind is empty, and no picture was taken; the
    /// preview and the sealed body agree, and both hold only what is there.
    #[test]
    fn a_ticked_box_with_nothing_behind_it_puts_nothing_in_the_report() {
        let lobby = Gathered {
            build: build(),
            system: Some(system()),
            settings: Some(Settings::default()),
            log: Some(SeatLog {
                seat: 0,
                roster: Vec::new(),
                lines: Vec::new(),
            }),
            ..Gathered::default()
        };
        let everything = Consent::everything();
        for category in [Category::Game, Category::Log, Category::Screenshot] {
            assert!(!lobby.has(category), "{category:?} has something");
            assert!(everything.allows(category), "{category:?} is ticked");
        }
        let body = json_of(lobby.submission(Kind::Bug, "x", &everything));
        let client = body["client"].as_object().expect("client is an object");
        let mut keys: Vec<_> = client.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["build", "settings", "system"]);

        let preview = ReportForm {
            text: crate::textbuf::TextBuffer::new("x"),
            ..ReportForm::default()
        }
        .preview_text(&lobby, &everything, Via::Gateway);
        let shown: serde_json::Value = serde_json::from_str(&preview).expect("the preview is JSON");
        for absent in ["game", "log", "screenshot"] {
            assert!(
                shown["client"].get(absent).is_none(),
                "the preview shows {absent}"
            );
        }
    }

    /// The whole point of [`seal`]. A token that reached the report by *any*
    /// route — here the free text, which is the one field no schema can
    /// constrain, because a player may paste anything into it — refuses the
    /// report instead of sending it.
    #[test]
    fn a_secret_anywhere_in_it_refuses_the_report() {
        let text = format!("ich war auf dem Tisch mit token={TOKEN}");
        let refused = gathered()
            .submission(Kind::Bug, &text, &Consent::default())
            .sealed(&token())
            .expect_err("the token is in there");
        assert_eq!(
            refused,
            Unsendable::Leaked(Leaked {
                label: "seat token".into()
            })
        );
        assert!(
            !refused.text(crate::i18n::Lang::En).contains(TOKEN),
            "a refusal that quoted the secret would be a way to read it"
        );
    }

    /// And it searches every field, not the text ones: a secret that arrived
    /// through a structured part — a gateway URL with a token in its query,
    /// pasted into an adapter string, which is how one would get there — is
    /// caught the same way.
    #[test]
    fn a_secret_in_a_structured_field_is_found_too() {
        let mut gathered = gathered();
        gathered.system.as_mut().expect("system").adapter =
            Some(format!("http://gateway/?token={TOKEN}"));
        let everything = Consent::everything();
        assert!(matches!(
            gathered
                .submission(Kind::Bug, "x", &everything)
                .sealed(&token()),
            Err(Unsendable::Leaked(_))
        ));
        // Unticked, it is not in the report, so there is nothing to refuse.
        gathered
            .submission(Kind::Bug, "x", &Consent::default())
            .sealed(&token())
            .expect("the system part was not sent");
    }

    /// Each kind of token the keyring holds refuses a report that carries
    /// it, under its own label, and a keyring holding all three finds each.
    #[test]
    fn the_keyring_refuses_each_kind_of_token_it_holds() {
        let session = "5e55105e55105e55105e55105e55105e";
        let seat = "5ea75ea75ea75ea75ea75ea75ea75ea7";
        let guest = "9ue579ue579ue579ue579ue579ue579u";
        let keyring = Keyring {
            sessions: vec![session.into()],
            seats: vec![seat.into()],
            guests: vec!["some other guest's token".into(), guest.into()],
        };
        for (token, label) in [
            (session, Keyring::SESSION),
            (seat, Keyring::SEAT),
            (guest, Keyring::GUEST),
        ] {
            let refused = gathered()
                .submission(
                    Kind::Bug,
                    &format!("pasted {token} by mistake"),
                    &Consent::default(),
                )
                .sealed(&keyring.secrets());
            assert_eq!(
                refused.map(|_| ()),
                Err(Unsendable::Leaked(Leaked {
                    label: label.into()
                })),
                "{label}"
            );
        }
        gathered()
            .submission(Kind::Bug, "nothing of theirs", &Consent::everything())
            .sealed(&keyring.secrets())
            .expect("a clean report passes the whole keyring");
        assert!(
            !format!("{keyring:?}").contains(session),
            "a keyring's Debug is counts, never tokens"
        );
    }

    /// A short "secret" is not one, and searching for it would refuse every
    /// report a player ever wrote: `"de"` is in the language field of all of
    /// them.
    #[test]
    fn something_too_short_to_be_a_secret_is_not_searched_for() {
        seal(
            &gathered().report(&Consent::everything()),
            &[
                Secret {
                    label: "language",
                    value: "de",
                },
                Secret {
                    label: "nothing at all",
                    value: "",
                },
            ],
        )
        .expect("neither is a secret");
    }

    /// The text limit is characters, as the gateway counts them: a text of
    /// umlauts at the limit goes, one more character does not.
    #[test]
    fn the_text_limit_counts_characters_not_bytes() {
        let at = "ä".repeat(MAX_TEXT_CHARS);
        gathered()
            .submission(Kind::Bug, &at, &Consent::default())
            .sealed(&[])
            .expect("exactly at the limit");
        let over = format!("{at}a");
        assert_eq!(
            gathered()
                .submission(Kind::Bug, &over, &Consent::default())
                .sealed(&[])
                .map(|_| ()),
            Err(Unsendable::TextTooLong)
        );
    }

    /// Over the size limit the screenshot goes first, then the log's oldest
    /// lines, and the report still goes out under the limit.
    #[test]
    fn an_oversized_report_drops_the_screenshot_then_the_oldest_log_lines() {
        let mut gathered = gathered();
        gathered.screenshot.as_mut().expect("shot").png_base64 = "A".repeat(MAX_CLIENT_BYTES);
        let (fitted, trimmed) = gathered
            .submission(Kind::Bug, "x", &Consent::everything())
            .fitted()
            .expect("fits without the picture");
        assert!(trimmed.screenshot && trimmed.log_lines.is_none());
        assert!(fitted.client.screenshot.is_none() && fitted.client.log.is_some());

        let rows = 40_000;
        gathered.log.as_mut().expect("log").lines = (0..rows)
            .map(|i| LogRow {
                turn: i,
                at: 0,
                text: format!("line {i:05} {}", "x".repeat(60)),
            })
            .collect();
        let (fitted, trimmed) = gathered
            .submission(Kind::Bug, "x", &Consent::everything())
            .fitted()
            .expect("fits with a shorter log");
        let kept = trimmed.log_lines.expect("the log was cut");
        assert!(kept > 0 && kept < rows as usize);
        let lines = &fitted.client.log.as_ref().expect("log").lines;
        assert_eq!(lines.len(), kept);
        assert_eq!(
            lines.last().map(|row| row.turn),
            Some(rows - 1),
            "the newest line is the one kept"
        );
        assert!(serde_json::to_vec(&fitted.client).expect("json").len() <= MAX_CLIENT_BYTES);
    }

    const RECORD: &str = concat!(
        r#"{"kind":"header","record":1,"build":"0.1.0","preset":{},"hash":"00"}"#,
        "\n",
        r#"{"kind":"input","n":0,"at":0,"seat":0,"by":"seat","action":"Pass","hash":"01"}"#,
        "\n",
    );

    /// A game this device hosted: no game id, and its record packed.
    fn local() -> Gathered {
        Gathered {
            game_id: None,
            local_record: LocalRecord::pack(RECORD.as_bytes()),
            ..gathered()
        }
    }

    /// The record rides only when the device has not said never and the
    /// player ticked it for this report; never under a networked game, and
    /// never by any door but the form's.
    #[test]
    fn a_local_record_rides_only_on_a_yes_for_this_report() {
        let ask = Consent::everything();
        let never = Consent {
            record: RecordConsent::Never,
            ..Consent::everything()
        };
        let with = json_of(local().submission_with(Kind::Bug, "x", &ask, true));
        let record = &with["local_record"];
        assert_eq!(record["complete"], false);
        assert!(
            record["gzip_base64"]
                .as_str()
                .is_some_and(|b| !b.is_empty())
        );
        assert_eq!(with["game_id"], serde_json::Value::Null);
        assert!(local().offers_record(&ask));

        for (consent, yes, why) in [
            (&ask, false, "not ticked"),
            (&never, true, "never"),
            (&never, false, "never, not ticked"),
        ] {
            let body = json_of(local().submission_with(Kind::Bug, "x", consent, yes));
            assert!(body.get("local_record").is_none(), "{why}");
        }
        assert!(!local().offers_record(&never));
        let body = json_of(local().submission(Kind::Bug, "x", &ask));
        assert!(
            body.get("local_record").is_none(),
            "the plain door sends none"
        );

        // A networked game's record is its gateway's: a local one beside it
        // is never offered, never sent, and the game id stays.
        let networked = Gathered {
            local_record: LocalRecord::pack(RECORD.as_bytes()),
            ..gathered()
        };
        assert!(!networked.offers_record(&ask));
        let body = json_of(networked.submission_with(Kind::Bug, "x", &ask, true));
        assert!(body.get("local_record").is_none());
        assert_eq!(body["game_id"], "g-42");
    }

    /// The leak check reads the record's own lines, which the base64 of
    /// its gzip would hide from a search of the body.
    #[test]
    fn a_secret_inside_a_record_blocks_the_send() {
        let token = "s3cr3t-session-token-0001";
        let leaky = Gathered {
            local_record: LocalRecord::pack(format!("{RECORD}{token}\n").as_bytes()),
            ..local()
        };
        let secrets = [Secret {
            label: Keyring::SESSION,
            value: token,
        }];
        let ask = Consent::default();
        let body = leaky.submission_with(Kind::Bug, "x", &ask, true);
        assert!(
            !serde_json::to_string(&body).expect("json").contains(token),
            "the body alone would pass"
        );
        for sealed in [
            body.clone().sealed(&secrets),
            body.sealed_direct(&"a".repeat(32), &secrets),
        ] {
            assert_eq!(
                sealed.map(|_| ()),
                Err(Unsendable::Leaked(Leaked {
                    label: Keyring::SESSION.into()
                }))
            );
        }
        assert!(
            leaky
                .submission_with(Kind::Bug, "x", &ask, false)
                .sealed(&secrets)
                .is_ok(),
            "without the record it goes"
        );
    }

    /// A record too large to ride stays home and the words still go.
    #[test]
    fn a_record_too_large_is_left_out_not_the_report() {
        use std::fmt::Write as _;
        let mut big = String::from(RECORD);
        let mut n = 0u64;
        // Hex of a scrambled counter compresses poorly: past the bound
        // even gzipped.
        while big.len() < MAX_RECORD_BYTES * 3 {
            n += 1;
            let noise = n.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ (n << 29);
            let _ = writeln!(big, "{noise:016x}{:016x}", noise.rotate_left(17));
        }
        let gathered = Gathered {
            local_record: LocalRecord::pack(big.as_bytes()),
            ..local()
        };
        assert!(!gathered.local_record.as_ref().expect("packed").fits());
        let (fitted, trimmed) = gathered
            .submission_with(Kind::Bug, "x", &Consent::default(), true)
            .fitted()
            .expect("the words fit");
        assert!(trimmed.record && fitted.local_record.is_none());
    }

    /// Straight to the service: the service's own shape (no game id, the
    /// build beside the parts, the device id), held to its smaller bound;
    /// through a gateway, never the device id.
    #[test]
    fn a_direct_report_is_the_services_shape_and_bound() {
        let device = "0123456789abcdef0123456789abcdef";
        let (json, _) = local()
            .submission_with(Kind::Feedback, told(), &Consent::everything(), true)
            .sealed_direct(device, &[])
            .expect("sendable");
        let body: serde_json::Value = serde_json::from_str(&json).expect("json");
        let mut keys: Vec<&str> = body
            .as_object()
            .expect("an object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["build", "client", "device", "kind", "record", "text"]
        );
        assert_eq!(body["device"], device);
        assert_eq!(body["build"], serde_json::to_value(build()).expect("json"));
        assert_eq!(body["kind"], "feedback");

        let (gateway, _) = local()
            .submission_with(Kind::Feedback, told(), &Consent::everything(), true)
            .sealed(&[])
            .expect("sendable");
        assert!(!gateway.contains(device) && !gateway.contains("\"device\""));

        let mut heavy = local();
        heavy.screenshot = Some(Screenshot {
            width: 1,
            height: 1,
            png_base64: "A".repeat(MAX_DIRECT_CLIENT_BYTES),
        });
        let (fitted, trimmed) = heavy
            .submission(Kind::Bug, "x", &Consent::everything())
            .fitted_to(MAX_DIRECT_CLIENT_BYTES)
            .expect("fits");
        assert!(trimmed.screenshot);
        assert!(serde_json::to_vec(&fitted.client).expect("json").len() <= MAX_DIRECT_CLIENT_BYTES);
        assert!(
            heavy
                .submission(Kind::Bug, "x", &Consent::everything())
                .fitted()
                .is_ok_and(|(_, t)| !t.screenshot),
            "a gateway's bound takes it whole"
        );
    }
}
