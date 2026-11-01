//! Filing a bug or crash report with Central (baylee-central `docs/reports.md`
//! §"Intake: native listener, raw bodies"): the metadata as JSON on
//! `POST /v1/reports`, then each part it announced as the raw gzip on
//! `PUT /v1/reports/{id}/client` and `/record`, within the hour.
//!
//! The shapes mirror `central_api::reports::ReportIntake` (Central #58),
//! which the pinned `central-api` predates; Central reads them with
//! `deny_unknown_fields` and refuses the whole report when one field is out
//! of its bounds, so [`ReportIntake::validate`] holds the same rules here and
//! the cleaners ([`shown`], [`said`]) bring what a client or an operator
//! wrote within them first. Central never unpacks a part: it checks the
//! announced length and SHA-256.

use serde::Serialize;

/// The largest metadata body Central reads.
pub const MAX_INTAKE_BYTES: usize = 65_536;
/// The longest report text, in characters.
pub const MAX_TEXT: usize = 20_000;
/// The longest game id, in characters (printable ASCII only).
pub const MAX_GAME_ID: usize = 128;
/// The longest reporter pseudonym, hex digits.
pub const MAX_REPORTER: usize = 128;
/// The longest thing a gateway says of itself, and a build's version: bytes and characters.
pub const MAX_SAID: usize = 256;
/// The longest commit, hex digits.
pub const MAX_COMMIT: usize = 64;
/// The longest crash message, in characters (lines allowed).
pub const MAX_CRASH_MESSAGE: usize = 2_000;
/// The longest crash location, in characters.
pub const MAX_CRASH_LOCATION: usize = 512;
/// The largest `client` part, gzip.
pub const MAX_CLIENT_GZIP: u32 = 2 * 1024 * 1024;
/// The largest record a gateway kept, gzip.
pub const MAX_RECORD_GZIP: u32 = 64 * 1024 * 1024;
/// The largest record a client wrote of its own game, gzip.
pub const MAX_CLIENT_RECORD_GZIP: u32 = 4 * 1024 * 1024;

/// What a report is about. A reported chat message is no report here: Central
/// refuses it, and it stays the gateway's moderation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportKind {
    /// Something is broken.
    Bug,
    /// Something could be better.
    Improvement,
    /// Anything the player wants to say.
    Feedback,
    /// The client crashed.
    Crash,
    /// None of these.
    Other,
}

/// How a report came: through a gateway's signed-in player, or straight from
/// a client to the official gateway.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// A signed-in player's, through their gateway.
    Gateway,
    /// A client signed in nowhere, through the official gateway only.
    Direct,
}

/// Who wrote a record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordOrigin {
    /// The gateway kept it from its engine.
    Gateway,
    /// The client wrote it of a game it hosted itself; unverified.
    Client,
}

/// A part of a report, sent after its metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// What the client said, as gzip of its JSON.
    Client,
    /// A game's record, gzip.
    Record,
}

impl Part {
    /// The word in the part's address.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Client => "client",
            Self::Record => "record",
        }
    }
}

/// What the gateway says of itself; Central stores it beside the name and
/// address it knows the gateway by.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct GatewaySays {
    /// Its name.
    pub name: Option<String>,
    /// Its address.
    pub url: Option<String>,
    /// Its build.
    pub version: Option<String>,
}

/// The client's build, as the report's metadata carries it for the list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Build {
    /// Its version.
    pub version: String,
    /// Its commit, hex.
    pub commit: Option<String>,
}

/// A crash's message and place, for crash groups.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Crash {
    /// The message, lines allowed.
    pub message: String,
    /// Where: a file and a line.
    pub location: String,
}

/// A part the report will send: its length and SHA-256.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Announced {
    /// Its length in bytes, gzip.
    pub bytes: u32,
    /// Its SHA-256.
    pub sha256: [u8; 32],
}

/// The record the report will send.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AnnouncedRecord {
    /// Whether the game's end is in it.
    pub complete: bool,
    /// Who wrote it.
    pub origin: RecordOrigin,
    /// Its length in bytes, gzip.
    pub bytes: u32,
    /// Its SHA-256.
    pub sha256: [u8; 32],
}

impl Announced {
    /// `gzip`'s announcement, when it fits a `u32`.
    #[must_use]
    pub fn of(gzip: &[u8]) -> Option<Self> {
        use sha2::Digest as _;
        Some(Self {
            bytes: u32::try_from(gzip.len()).ok()?,
            sha256: sha2::Sha256::digest(gzip).into(),
        })
    }
}

/// A report's metadata, as `POST /v1/reports` takes it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReportIntake {
    /// What the gateway says of itself.
    pub gateway: GatewaySays,
    /// How it came.
    pub channel: Channel,
    /// The reporter's pseudonym, hex.
    pub reporter: String,
    /// What it is about.
    pub kind: ReportKind,
    /// What the player wrote.
    pub text: String,
    /// The gateway's game it is about.
    pub game_id: Option<String>,
    /// The client's build.
    pub build: Option<Build>,
    /// A crash's message and place, only for a crash.
    pub crash: Option<Crash>,
    /// The `client` part it will send.
    pub client: Option<Announced>,
    /// The record it will send.
    pub record: Option<AnnouncedRecord>,
}

/// The invisible characters Central refuses because they change how the text
/// around them reads (direction marks, embeddings, overrides, isolates,
/// invisible separators and spaces, the byte order mark); the zero-width
/// joiners stay (`central_api::reports`).
const fn invisible(c: char) -> bool {
    matches!(
        c,
        '\u{061C}'
            | '\u{200B}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}'
            | '\u{FEFF}'
    )
}

/// Whether Central shows `c`: no control character (a line break and a tab
/// where `lines`), nothing [`invisible`].
const fn kept(c: char, lines: bool) -> bool {
    (lines && (c == '\n' || c == '\t')) || (!c.is_control() && !invisible(c))
}

/// Whether `text` is within `max` characters and Central shows all of it.
fn is_shown(text: &str, max: usize, lines: bool) -> bool {
    text.chars().count() <= max && text.chars().all(|c| kept(c, lines))
}

/// `text` as Central keeps it: a carriage return before a line break dropped
/// and a lone one made a line break (where `lines`), every other character
/// Central refuses left out, and cut to `max` characters.
#[must_use]
pub fn shown(text: &str, max: usize, lines: bool) -> String {
    let mut out = String::with_capacity(text.len().min(max * 4));
    let mut chars = text.chars().peekable();
    let mut count = 0;
    while let Some(mut c) = chars.next() {
        if c == '\r' && lines {
            if chars.peek() == Some(&'\n') {
                continue;
            }
            c = '\n';
        }
        if !kept(c, lines) {
            continue;
        }
        if count == max {
            break;
        }
        out.push(c);
        count += 1;
    }
    out
}

/// One line a gateway says of itself or a build's version: [`shown`] without
/// line breaks, trimmed, cut to [`MAX_SAID`] bytes on a character's edge;
/// `None` when nothing is left.
#[must_use]
pub fn said(text: &str) -> Option<String> {
    let mut out = shown(text, MAX_SAID, false).trim().to_owned();
    while out.len() > MAX_SAID {
        out.pop();
    }
    let out = out.trim_end().to_owned();
    (!out.is_empty()).then_some(out)
}

fn said_ok(text: Option<&str>) -> bool {
    text.is_none_or(|t| !t.is_empty() && t.len() <= MAX_SAID && is_shown(t, MAX_SAID, false))
}

impl ReportIntake {
    /// Central's own check (`central_api::reports::ReportIntake::validate`):
    /// whether Central would take this report as it stands.
    #[must_use]
    pub fn validate(&self) -> bool {
        let reporter = !self.reporter.is_empty()
            && self.reporter.len() <= MAX_REPORTER
            && self.reporter.bytes().all(|b| b.is_ascii_hexdigit());
        let text = !self.text.trim().is_empty() && is_shown(&self.text, MAX_TEXT, true);
        let game = self.game_id.as_deref().is_none_or(|g| {
            !g.is_empty()
                && g.chars().count() <= MAX_GAME_ID
                && g.chars().all(|c| c.is_ascii_graphic())
        });
        let build = self.build.as_ref().is_none_or(|b| {
            !b.version.is_empty()
                && b.version.len() <= MAX_SAID
                && is_shown(&b.version, MAX_SAID, false)
                && b.commit.as_deref().is_none_or(|c| {
                    !c.is_empty()
                        && c.len() <= MAX_COMMIT
                        && c.bytes().all(|b| b.is_ascii_hexdigit())
                })
        });
        let crash = self.crash.as_ref().is_none_or(|c| {
            is_shown(&c.message, MAX_CRASH_MESSAGE, true)
                && is_shown(&c.location, MAX_CRASH_LOCATION, false)
        });
        let client = self
            .client
            .as_ref()
            .is_none_or(|p| p.bytes > 0 && p.bytes <= MAX_CLIENT_GZIP);
        let record = self.record.as_ref().is_none_or(|r| {
            r.bytes > 0
                && r.bytes
                    <= match r.origin {
                        RecordOrigin::Gateway => MAX_RECORD_GZIP,
                        RecordOrigin::Client => MAX_CLIENT_RECORD_GZIP,
                    }
        });
        let consistent = (self.channel == Channel::Gateway || self.game_id.is_none())
            && (self.crash.is_none() || self.kind == ReportKind::Crash);
        reporter
            && text
            && game
            && build
            && crash
            && client
            && record
            && consistent
            && said_ok(self.gateway.name.as_deref())
            && said_ok(self.gateway.url.as_deref())
            && said_ok(self.gateway.version.as_deref())
    }
}

/// The client's build as its `client` object names it (`build {version,
/// commit}`): the version as one line within bounds, the commit only as hex
/// digits; `None` when it names no version.
#[must_use]
pub fn build_of(client: &serde_json::Value) -> Option<Build> {
    let build = client.get("build")?;
    let version = said(build.get("version")?.as_str()?)?;
    let commit = build
        .get("commit")
        .and_then(serde_json::Value::as_str)
        .filter(|c| {
            !c.is_empty() && c.len() <= MAX_COMMIT && c.bytes().all(|b| b.is_ascii_hexdigit())
        })
        .map(str::to_owned);
    Some(Build { version, commit })
}

/// A crash's message and place as the `client` object names them (`crash
/// {message, location}`), cut to Central's bounds; `None` when it names
/// neither. Only a crash report carries one.
#[must_use]
pub fn crash_of(client: &serde_json::Value) -> Option<Crash> {
    let crash = client.get("crash")?;
    let text = |key: &str| crash.get(key).and_then(serde_json::Value::as_str);
    let (message, location) = (text("message"), text("location"));
    if message.is_none() && location.is_none() {
        return None;
    }
    Some(Crash {
        message: shown(message.unwrap_or_default(), MAX_CRASH_MESSAGE, true),
        location: shown(location.unwrap_or_default(), MAX_CRASH_LOCATION, false),
    })
}

/// A report's text as Central takes it ([`shown`]); a crash report left with
/// none reads its crash's message, or says only what it is. `None` for any
/// other report left with none.
#[must_use]
pub fn text_of(text: &str, kind: ReportKind, crash: Option<&Crash>) -> Option<String> {
    let text = shown(text, MAX_TEXT, true);
    if !text.trim().is_empty() {
        return Some(text);
    }
    (kind == ReportKind::Crash).then(|| {
        crash
            .map(|c| c.message.trim().to_owned())
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| "The client crashed.".to_owned())
    })
}

/// A report id as Central writes it: 32 lowercase hex digits.
pub(crate) fn id_from_hex(text: &str) -> Option<[u8; 16]> {
    if text.len() != 32 || !text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let mut id = [0_u8; 16];
    for (i, byte) in id.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(id)
}

/// `id` as Central writes it.
#[must_use]
pub fn id_hex(id: &[u8; 16]) -> String {
    use std::fmt::Write as _;
    id.iter().fold(String::with_capacity(32), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intake() -> ReportIntake {
        ReportIntake {
            gateway: GatewaySays {
                name: Some("Synthetic gateway".into()),
                url: Some("https://gateway.example".into()),
                version: Some("0.1.1-beta.9b".into()),
            },
            channel: Channel::Gateway,
            reporter: "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843".into(),
            kind: ReportKind::Crash,
            text: "It crashed\nwhile I was choosing attackers".into(),
            game_id: Some("0192d0c5-7b50-7a1f-8c2d-3e4f5a6b7c01".into()),
            build: Some(Build {
                version: "0.1.1-beta.9b".into(),
                commit: Some("3f9a1c7e21".into()),
            }),
            crash: Some(Crash {
                message: "index out of bounds: the len is 3 but the index is 7".into(),
                location: "crates/baylee-client/src/table.rs:812:17".into(),
            }),
            client: Some(Announced {
                bytes: 182,
                sha256: [7; 32],
            }),
            record: Some(AnnouncedRecord {
                complete: false,
                origin: RecordOrigin::Gateway,
                bytes: 4_000,
                sha256: [9; 32],
            }),
        }
    }

    /// The JSON Central reads, field for field: `central_api::reports`
    /// at Central #58 decodes exactly this (checked against Central's own
    /// type when this mirror was written, 31.10.2026).
    #[test]
    fn the_metadata_is_centrals_shape() {
        let json = serde_json::to_value(intake()).unwrap();
        let mut keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "build", "channel", "client", "crash", "game_id", "gateway", "kind", "record",
                "reporter", "text"
            ]
        );
        assert_eq!(json["channel"], "gateway");
        assert_eq!(json["kind"], "crash");
        assert_eq!(json["record"]["origin"], "gateway");
        assert_eq!(json["client"]["sha256"].as_array().unwrap().len(), 32);
        assert_eq!(json["gateway"]["url"], "https://gateway.example");
        assert!(intake().validate());
    }

    #[test]
    fn central_refuses_what_it_does_not_show() {
        let refused = [
            ReportIntake {
                text: "  \n ".into(),
                ..intake()
            },
            ReportIntake {
                text: "over\u{202E}ride".into(),
                ..intake()
            },
            ReportIntake {
                text: "a\r\nb".into(),
                ..intake()
            },
            ReportIntake {
                reporter: "not hex".into(),
                ..intake()
            },
            ReportIntake {
                game_id: Some("with space".into()),
                ..intake()
            },
            ReportIntake {
                kind: ReportKind::Bug,
                ..intake()
            },
            ReportIntake {
                channel: Channel::Direct,
                ..intake()
            },
            ReportIntake {
                gateway: GatewaySays {
                    name: Some(String::new()),
                    ..GatewaySays::default()
                },
                ..intake()
            },
            ReportIntake {
                build: Some(Build {
                    version: "1".into(),
                    commit: Some("xyz".into()),
                }),
                ..intake()
            },
            ReportIntake {
                record: Some(AnnouncedRecord {
                    complete: true,
                    origin: RecordOrigin::Client,
                    bytes: MAX_CLIENT_RECORD_GZIP + 1,
                    sha256: [0; 32],
                }),
                ..intake()
            },
        ];
        for report in refused {
            assert!(!report.validate(), "{report:?}");
        }
    }

    #[test]
    fn the_cleaners_bring_text_within_centrals_rules() {
        assert_eq!(shown("a\r\nb\rc", 100, true), "a\nb\nc");
        assert_eq!(
            shown("a\u{200F}b\u{0007}c\u{200D}d", 100, true),
            "abc\u{200D}d"
        );
        assert_eq!(shown("line\nbreak\ttab", 100, false), "linebreaktab");
        assert_eq!(shown("ééééé", 3, true), "ééé");
        assert!(is_shown(
            &shown("x\u{202E}\r\ny", MAX_TEXT, true),
            MAX_TEXT,
            true
        ));
        assert_eq!(said("  \u{200E} "), None);
        assert_eq!(said(" Name\n"), Some("Name".into()));
        let long = said(&"é".repeat(300)).unwrap();
        assert!(long.len() <= MAX_SAID && long.chars().all(|c| c == 'é'));
    }

    #[test]
    fn ids_read_as_central_writes_them() {
        let id = [0xab; 16];
        assert_eq!(id_from_hex(&id_hex(&id)), Some(id));
        assert_eq!(id_from_hex("AB"), None);
        assert_eq!(id_from_hex(&"G".repeat(32)), None);
    }
}
