//! A replay's tape: the frames one perspective of a game is sent, as gzip
//! JSON Lines (`docs/protocol.md` §"Replays").
//!
//! The engine-server writes one for a hosted game the gateway asks it to
//! project, the client writes one in memory for a game it hosted against the
//! house, and the client reads both; this crate is the one all three link.
//! Pure: types and one line at a time. Gzip is the caller's.
//!
//! ```text
//! {"kind":"tape","tape":1,"view":76,"build":"…","source":"hosted","perspective":"seat","seat":1,"game":"…","recorded_by":"…"}
//! {"kind":"static","at":1791000000040,"json":{…GameStatic…}}
//! {"kind":"view","at":1791000001210,"n":null,"check":"exact","seq":3,"json":{…PlayerView…},"log":{…LogTail…}}
//! {"kind":"curtain","at":…}
//! {"kind":"over","at":…,"json":{"GameOver":{…}}}
//! {"kind":"stop","stop":"end","inputs":812,"exact_to":null,"seen_to":811,"diverged":null}
//! ```
//!
//! The embedded payloads (`json`, `log`) are the wire's own JSON, kept raw
//! ([`RawValue`]) and never re-encoded: a reader decodes a view only when it
//! shows it. A tape holds only these kinds: never a seat's questions, its
//! `AiLog`, spectators, tokens or chat.
//!
//! The first line is the [`Header`]; the last is the [`Stop`]. A tape
//! without a `stop` line was cut in transit.

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

/// The shape of a tape; a reader refuses one it does not know.
pub const TAPE_VERSION: u32 = 1;

/// Where the game a tape tells came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// A game this computer hosted against the house, re-run here.
    Local,
    /// A hosted game, re-run on the gateway's side.
    Hosted,
    /// A record file opened by hand (`BAYLEE_REPLAY`).
    File,
}

/// Whose view of the game a tape is.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Perspective {
    /// One seat, exactly as a socket at it was told the game.
    Seat,
    /// The spectators' stream: nobody's hand.
    Public,
    /// One seat with every other hand beside its own; only for a record its
    /// holder already holds whole.
    AllHands,
}

/// How far a frame is known to be the recorded game's.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Check {
    /// The engine's hash was the record's: every bit (same build).
    Exact,
    /// The public table's digest (`seen`) was the record's.
    Seen,
    /// Neither holds, or the record has nothing to check against.
    Unverified,
}

/// Why a tape ends where it does.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopKind {
    /// The game's end.
    End,
    /// The record ends without an end: its engine died, or it was cut.
    Cut,
    /// A check failed and the projection was asked to stop there.
    Diverged,
    /// This build's engine refused one of the record's inputs.
    Refused,
    /// The projection was cancelled.
    Cancelled,
    /// The tape grew past its bound.
    TooLarge,
}

/// Which check found a divergence.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    /// The engine's hash (the same build).
    Hash,
    /// The public table's digest (another build).
    Seen,
}

/// Where the replayed game first differed from the record.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Divergence {
    /// The record input after which it differed; `None` before the first.
    pub input: Option<u64>,
    /// Which check said so.
    pub by: Basis,
}

/// A tape's first line.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Header {
    /// [`TAPE_VERSION`].
    pub tape: u32,
    /// The `VIEW_VERSION` of every payload in the tape.
    pub view: u32,
    /// The build that projected it.
    pub build: String,
    /// Where the game came from.
    pub source: Source,
    /// Whose view it is.
    pub perspective: Perspective,
    /// The seat the table is drawn from; 0 for [`Perspective::Public`], as
    /// a spectator's socket.
    pub seat: u8,
    /// The gateway's game id, or the record's file name.
    pub game: String,
    /// The build that played and recorded the game (the record header's).
    pub recorded_by: String,
    /// Whether the drawn seat's own hidden cards are in the views (its hand,
    /// what it looked at, its face-down cards). `false`: the public table
    /// drawn from that seat, because it did not share them
    /// (`docs/protocol.md` §"Replays"). Absent reads `true`.
    #[serde(default = "shown", skip_serializing_if = "is_shown")]
    pub own: bool,
    /// The other seats whose hands are shown beside the drawn seat's (as
    /// `shared_hands`). Absent reads none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hands: Vec<u8>,
}

const fn shown() -> bool {
    true
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde's skip_serializing_if hands a reference
const fn is_shown(own: &bool) -> bool {
    *own
}

/// A `view` line: one `StateDelta` as the perspective was sent it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct View {
    /// The record's host time, milliseconds since the Unix epoch.
    pub at: u64,
    /// The record input after which the frame was made; `None` before the
    /// first.
    pub n: Option<u64>,
    /// How far it is known to be the recorded game's.
    pub check: Check,
    /// The delta's `seq`.
    pub seq: u64,
    /// The delta's `view_json`: a `PlayerView`, or a `SpectatorView` for
    /// [`Perspective::Public`].
    pub json: Box<RawValue>,
    /// The delta's `log_json` (a `LogTail`); `None` when it told nothing.
    pub log: Option<Box<RawValue>>,
}

/// The last line: how far the tape is the recorded game.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Stop {
    /// Why it ends here.
    pub stop: StopKind,
    /// The record inputs the projection applied.
    pub inputs: u64,
    /// The last input whose engine hash was the record's (`None`: not even
    /// the opening's).
    pub exact_to: Option<u64>,
    /// The last input whose `seen` was the record's.
    pub seen_to: Option<u64>,
    /// The first divergence, when one was found.
    pub diverged: Option<Divergence>,
}

impl Stop {
    /// A stop that says only why, with nothing applied or checked: what a
    /// writer sends when it could not project at all.
    #[must_use]
    pub const fn bare(stop: StopKind) -> Self {
        Self {
            stop,
            inputs: 0,
            exact_to: None,
            seen_to: None,
            diverged: None,
        }
    }
}

/// One line of a tape.
#[derive(Clone, Debug)]
pub enum Line {
    /// `tape`: the header, first.
    Header(Header),
    /// `static`: a `GameStaticMsg.static_json`.
    Static {
        /// The record's host time.
        at: u64,
        /// The `GameStatic`.
        json: Box<RawValue>,
    },
    /// `view`: a `StateDelta`.
    View(View),
    /// `curtain`: the table opened.
    Curtain {
        /// The record's host time.
        at: u64,
    },
    /// `over`: the game's end, as `Pending::GameOver`.
    Over {
        /// The record's host time.
        at: u64,
        /// The pending choice's JSON (`{"GameOver":{…}}`).
        json: Box<RawValue>,
    },
    /// `stop`: the last line.
    Stop(Stop),
}

/// Why a line is not a tape line.
#[derive(Debug)]
pub struct LineError(String);

impl std::fmt::Display for LineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LineError {}

impl From<serde_json::Error> for LineError {
    fn from(e: serde_json::Error) -> Self {
        Self(e.to_string())
    }
}

// The wire spelling: each kind is its own struct carrying its `kind`, so a
// line is written and read without serde's internally tagged enums, which
// buffer their content and cannot hold a `RawValue`.

#[derive(Serialize)]
struct Tagged<'a, T> {
    kind: &'static str,
    #[serde(flatten)]
    body: &'a T,
}

#[derive(Serialize, Deserialize)]
struct AtOnly {
    at: u64,
}

#[derive(Serialize, Deserialize)]
struct AtJson {
    at: u64,
    json: Box<RawValue>,
}

#[derive(Deserialize)]
struct KindOf<'a> {
    #[serde(borrow)]
    kind: &'a str,
}

/// One line as it is written, without its newline.
///
/// # Panics
/// Never: every field serializes (no map with non-string keys, no failing
/// `Serialize`).
#[must_use]
pub fn encode_line(line: &Line) -> String {
    fn tagged<T: Serialize>(kind: &'static str, body: &T) -> String {
        serde_json::to_string(&Tagged { kind, body }).expect("a tape line serializes")
    }
    match line {
        Line::Header(header) => tagged("tape", header),
        Line::Static { at, json } => tagged(
            "static",
            &AtJson {
                at: *at,
                json: json.clone(),
            },
        ),
        Line::View(view) => tagged("view", view),
        Line::Curtain { at } => tagged("curtain", &AtOnly { at: *at }),
        Line::Over { at, json } => tagged(
            "over",
            &AtJson {
                at: *at,
                json: json.clone(),
            },
        ),
        Line::Stop(stop) => tagged("stop", stop),
    }
}

/// Appends `line` and its newline to `out`.
pub fn write_line(out: &mut Vec<u8>, line: &Line) {
    out.extend_from_slice(encode_line(line).as_bytes());
    out.push(b'\n');
}

/// One line read back. Unknown fields are passed over (a later tape may
/// say more); an unknown `kind` is an error.
///
/// # Errors
/// The line is not JSON, names no known kind, or lacks a field its kind has.
pub fn parse_line(text: &str) -> Result<Line, LineError> {
    let KindOf { kind } = serde_json::from_str(text)?;
    Ok(match kind {
        "tape" => Line::Header(serde_json::from_str(text)?),
        "static" => {
            let AtJson { at, json } = serde_json::from_str(text)?;
            Line::Static { at, json }
        }
        "view" => Line::View(serde_json::from_str(text)?),
        "curtain" => {
            let AtOnly { at } = serde_json::from_str(text)?;
            Line::Curtain { at }
        }
        "over" => {
            let AtJson { at, json } = serde_json::from_str(text)?;
            Line::Over { at, json }
        }
        "stop" => Line::Stop(serde_json::from_str(text)?),
        other => return Err(LineError(format!("no tape line is of kind `{other}`"))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(json: &str) -> Box<RawValue> {
        RawValue::from_string(json.to_owned()).unwrap()
    }

    #[test]
    fn every_kind_reads_back_as_it_was_written() {
        let lines = [
            Line::Header(Header {
                tape: TAPE_VERSION,
                view: 76,
                build: "0.8.0+build.212 (3f9a1c7e21)".into(),
                source: Source::Hosted,
                perspective: Perspective::AllHands,
                seat: 1,
                game: "0199c8".into(),
                recorded_by: "0.8.0+build.198 (77b01e2a90)".into(),
                own: false,
                hands: vec![0, 2],
            }),
            Line::Static {
                at: 40,
                json: raw(r#"{"seats":[1, 2]}"#),
            },
            Line::View(View {
                at: 1210,
                n: None,
                check: Check::Exact,
                seq: 3,
                json: raw(r#"{"turn":{"number":1}}"#),
                log: Some(raw(r#"{"from":0,"entries":[]}"#)),
            }),
            Line::View(View {
                at: 4005,
                n: Some(17),
                check: Check::Seen,
                seq: 4,
                json: raw("{}"),
                log: None,
            }),
            Line::Curtain { at: 50 },
            Line::Over {
                at: 9300,
                json: raw(r#"{"GameOver":{"winners":[0]}}"#),
            },
            Line::Stop(Stop {
                stop: StopKind::Diverged,
                inputs: 812,
                exact_to: None,
                seen_to: Some(811),
                diverged: Some(Divergence {
                    input: Some(812),
                    by: Basis::Seen,
                }),
            }),
        ];
        let mut out = Vec::new();
        for line in &lines {
            write_line(&mut out, line);
        }
        let text = String::from_utf8(out).unwrap();
        let read: Vec<Line> = text.lines().map(|l| parse_line(l).unwrap()).collect();
        assert_eq!(read.len(), lines.len());
        for (a, b) in lines.iter().zip(&read) {
            assert_eq!(encode_line(a), encode_line(b));
        }
        // The payloads are carried byte for byte, spaces and all.
        assert!(text.contains(r#""json":{"seats":[1, 2]}"#), "{text}");
        assert!(
            text.starts_with(
                r#"{"kind":"tape","tape":1,"view":76,"build":"0.8.0+build.212 (3f9a1c7e21)","source":"hosted","perspective":"all_hands","seat":1,"#
            ),
            "{text}"
        );
        assert!(
            text.contains(
                r#"{"kind":"view","at":4005,"n":17,"check":"seen","seq":4,"json":{},"log":null}"#
            ),
            "{text}"
        );
        assert!(
            text.contains(
                r#"{"kind":"stop","stop":"diverged","inputs":812,"exact_to":null,"seen_to":811,"diverged":{"input":812,"by":"seen"}}"#
            ),
            "{text}"
        );
    }

    /// A header written before `own` and `hands` existed: the whole table
    /// of the drawn seat, no other hand.
    const OLD_HEADER: &str = r#"{"kind":"tape","tape":1,"view":76,"build":"b","source":"hosted","perspective":"seat","seat":1,"game":"g","recorded_by":"r"}"#;

    fn header_of(text: &str) -> Header {
        match parse_line(text).expect("a header") {
            Line::Header(header) => header,
            other => panic!("not a header: {}", encode_line(&other)),
        }
    }

    fn a_header(own: bool, hands: Vec<u8>) -> Header {
        Header {
            tape: TAPE_VERSION,
            view: 76,
            build: "b".into(),
            source: Source::Hosted,
            perspective: Perspective::Seat,
            seat: 1,
            game: "g".into(),
            recorded_by: "r".into(),
            own,
            hands,
        }
    }

    #[test]
    fn a_header_without_own_and_hands_reads_as_the_seats_own_and_no_hand() {
        let header = header_of(OLD_HEADER);
        assert!(
            header.own,
            "absent `own` is true: the drawn seat's own cards"
        );
        assert!(header.hands.is_empty(), "absent `hands` is none");
        // And what it writes back is the header it was, byte for byte.
        assert_eq!(encode_line(&Line::Header(header)), OLD_HEADER);
        // An explicit `own: true` reads the same.
        let explicit = OLD_HEADER.replace(r#""seat":1,"#, r#""seat":1,"own":true,"hands":[],"#);
        let header = header_of(&explicit);
        assert!(header.own && header.hands.is_empty());
    }

    #[test]
    fn a_header_that_withholds_nothing_does_not_write_own_or_hands() {
        let text = encode_line(&Line::Header(a_header(true, Vec::new())));
        assert!(
            !text.contains("\"own\"") && !text.contains("\"hands\""),
            "{text}"
        );
        assert_eq!(text, OLD_HEADER);
        // Hands alone are written, own (true) still is not.
        let text = encode_line(&Line::Header(a_header(true, vec![0])));
        assert!(
            text.contains(r#""hands":[0]"#) && !text.contains("\"own\""),
            "{text}"
        );
        // Own alone: withheld, no hands.
        let text = encode_line(&Line::Header(a_header(false, Vec::new())));
        assert!(
            text.contains(r#""own":false"#) && !text.contains("\"hands\""),
            "{text}"
        );
    }

    #[test]
    fn a_header_that_withholds_the_seats_own_cards_or_shows_hands_reads_back() {
        for (own, hands) in [
            (false, Vec::new()),
            (false, vec![0, 2]),
            (true, vec![2]),
            (true, vec![0, 3, 15]),
        ] {
            let header = a_header(own, hands.clone());
            let text = encode_line(&Line::Header(header));
            let back = header_of(&text);
            assert_eq!((back.own, &back.hands), (own, &hands), "{text}");
            assert_eq!(encode_line(&Line::Header(back)), text);
        }
        // A reader of a later tape passes over what it does not know, own
        // and hands included.
        let later =
            OLD_HEADER.replace(r#""seat":1,"#, r#""seat":1,"own":false,"hands":[2],"x":1,"#);
        let header = header_of(&later);
        assert!(!header.own);
        assert_eq!(header.hands, vec![2]);
    }

    #[test]
    fn a_line_of_no_known_kind_is_refused() {
        assert!(parse_line(r#"{"kind":"input","at":1}"#).is_err());
        assert!(parse_line(r#"{"at":1}"#).is_err());
        assert!(parse_line("not json").is_err());
        assert!(parse_line(r#"{"kind":"view","at":1}"#).is_err());
    }
}
