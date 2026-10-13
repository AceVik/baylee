//! The tape's lines as they are written (§4).
//!
//! The one place that knows the line format. `baylee_protocol::tape` is its
//! home (the engine-server writes tapes, this crate reads them); until that
//! lands this file mirrors it, and afterwards it becomes that module's
//! re-export, so nothing outside it changes.
//!
//! A line is read through one struct with every field optional rather than a
//! tagged enum: serde buffers an internally tagged enum's content before it
//! picks the variant, and a [`RawValue`] cannot be read out of that buffer.
//! The payloads stay raw, as the wire wrote them, and are decoded only when a
//! moment is shown.

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

/// The tape format this client reads and writes.
pub const TAPE_VERSION: u32 = 1;

/// Where a tape came from.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// A game this computer hosted against the house, re-run here.
    Local,
    /// A hosted game, re-run by the gateway's engine.
    Hosted,
    /// A record or tape file opened by hand (`BAYLEE_REPLAY`).
    File,
}

/// How far a moment is known to be the recorded game's.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Check {
    /// The engine's hash was the record's: every bit as played.
    Exact,
    /// The public table was the record's (`seen`), on another build.
    Seen,
    /// Neither could be checked, or one differed and the player played on.
    Unverified,
}

/// The tape's first line.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Header {
    /// [`TAPE_VERSION`] of the writer.
    pub tape: u32,
    /// `VIEW_VERSION` of every payload.
    pub view: u32,
    /// The build that projected it.
    pub build: String,
    /// Where it came from.
    pub source: Source,
    /// `seat`, `public` or `all_hands`.
    pub perspective: String,
    /// The seat the table is drawn from (0 for public).
    pub seat: u8,
    /// The gateway's game id, or the record's file name.
    pub game: String,
    /// The build that recorded the game.
    #[serde(default)]
    pub recorded_by: String,
}

impl Header {
    /// The perspective the header names, `None` for a spelling this client
    /// does not know.
    #[must_use]
    pub fn perspective(&self) -> Option<super::Perspective> {
        let seat = baylee_core::ids::PlayerId::new(self.seat);
        match self.perspective.as_str() {
            "seat" => Some(super::Perspective::Seat(seat)),
            "public" => Some(super::Perspective::Public),
            "all_hands" => Some(super::Perspective::AllHands(seat)),
            _ => None,
        }
    }
}

/// Why a tape ends where it does.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The game's end.
    End,
    /// The record ends before the game does (or the tape was cut in
    /// transit: a tape without its `stop` line reads as this).
    Cut,
    /// This build would play it differently from here; stopped there.
    Diverged,
    /// This build refused one of the record's answers.
    Refused,
    /// The projection was cancelled.
    Cancelled,
    /// The tape outgrew what a tape may weigh.
    TooLarge,
}

/// Where a divergence was found, and by which check.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Diverged {
    /// The record input (`None`: the opening already differed).
    pub input: Option<u64>,
    /// `hash` or `seen`.
    pub by: DivergedBy,
}

/// Which check found a divergence.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DivergedBy {
    /// The engine's hash, on the recording build.
    Hash,
    /// The public table's digest.
    Seen,
}

/// The tape's last line: how far it is the recorded game.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StopLine {
    /// Why it ends there.
    pub stop: StopReason,
    /// The record's inputs played.
    #[serde(default)]
    pub inputs: u64,
    /// The last input whose hash was the record's.
    #[serde(default)]
    pub exact_to: Option<u64>,
    /// The last input whose `seen` was the record's.
    #[serde(default)]
    pub seen_to: Option<u64>,
    /// The first divergence noted.
    #[serde(default)]
    pub diverged: Option<Diverged>,
}

impl StopLine {
    /// What a tape without its `stop` line says: cut in transit.
    #[must_use]
    pub const fn cut() -> Self {
        Self {
            stop: StopReason::Cut,
            inputs: 0,
            exact_to: None,
            seen_to: None,
            diverged: None,
        }
    }
}

/// One line after the header, its payloads still raw.
#[derive(Debug)]
pub(super) enum Line {
    /// A `GameStatic` payload.
    Static { at: u64, json: Box<RawValue> },
    /// A view and the log lines that came with it.
    View {
        at: u64,
        n: Option<u64>,
        check: Check,
        seq: u64,
        json: Box<RawValue>,
        log: Option<Box<RawValue>>,
    },
    /// The table opened.
    Curtain { at: u64 },
    /// The `GameOver` question.
    Over { at: u64, json: Box<RawValue> },
    /// The last line.
    Stop(StopLine),
}

/// Every field any line has, all optional: see the module's doc.
#[derive(Deserialize)]
struct AnyLine {
    kind: String,
    #[serde(default)]
    at: Option<u64>,
    #[serde(default)]
    n: Option<u64>,
    #[serde(default)]
    check: Option<Check>,
    #[serde(default)]
    seq: Option<u64>,
    #[serde(default)]
    json: Option<Box<RawValue>>,
    #[serde(default)]
    log: Option<Box<RawValue>>,
    #[serde(default)]
    stop: Option<StopReason>,
    #[serde(default)]
    inputs: Option<u64>,
    #[serde(default)]
    exact_to: Option<u64>,
    #[serde(default)]
    seen_to: Option<u64>,
    #[serde(default)]
    diverged: Option<Diverged>,
}

/// Reads the header line. `None`: not a tape's header.
pub(super) fn parse_header(line: &[u8]) -> Option<Header> {
    #[derive(Deserialize)]
    struct Kind<'a> {
        kind: &'a str,
    }
    let kind: Kind = serde_json::from_slice(line).ok()?;
    if kind.kind != "tape" {
        return None;
    }
    serde_json::from_slice(line).ok()
}

/// The header's version, read without the rest, so another tape version is
/// named rather than called unreadable.
pub(super) fn header_version(line: &[u8]) -> Option<(u32, Option<u32>)> {
    #[derive(Deserialize)]
    struct Versions<'a> {
        kind: &'a str,
        tape: u32,
        #[serde(default)]
        view: Option<u32>,
    }
    let v: Versions = serde_json::from_slice(line).ok()?;
    (v.kind == "tape").then_some((v.tape, v.view))
}

/// Reads one line after the header. `Err(())`: not a line this format has.
pub(super) fn parse_line(line: &[u8]) -> Result<Line, ()> {
    let any: AnyLine = serde_json::from_slice(line).map_err(|_| ())?;
    let at = any.at.unwrap_or(0);
    Ok(match any.kind.as_str() {
        "static" => Line::Static {
            at,
            json: any.json.ok_or(())?,
        },
        "view" => Line::View {
            at,
            n: any.n,
            check: any.check.unwrap_or(Check::Unverified),
            seq: any.seq.ok_or(())?,
            json: any.json.ok_or(())?,
            // `null` reads as no log at all.
            log: any.log.filter(|raw| raw.get() != "null"),
        },
        "curtain" => Line::Curtain { at },
        "over" => Line::Over {
            at,
            json: any.json.ok_or(())?,
        },
        "stop" => Line::Stop(StopLine {
            stop: any.stop.ok_or(())?,
            inputs: any.inputs.unwrap_or(0),
            exact_to: any.exact_to,
            seen_to: any.seen_to,
            diverged: any.diverged,
        }),
        _ => return Err(()),
    })
}

/// Writes the header line, newline included.
pub(super) fn encode_header(header: &Header) -> Vec<u8> {
    #[derive(Serialize)]
    struct Tagged<'a> {
        kind: &'static str,
        #[serde(flatten)]
        header: &'a Header,
    }
    let mut out = serde_json::to_vec(&Tagged {
        kind: "tape",
        header,
    })
    .unwrap_or_default();
    out.push(b'\n');
    out
}

/// Writes one line, newline included.
pub(super) fn encode_line(line: &Line) -> Vec<u8> {
    let value = match line {
        Line::Static { at, json } => serde_json::json!({"kind": "static", "at": at, "json": json}),
        Line::View {
            at,
            n,
            check,
            seq,
            json,
            log,
        } => serde_json::json!({
            "kind": "view", "at": at, "n": n, "check": check, "seq": seq,
            "json": json, "log": log,
        }),
        Line::Curtain { at } => serde_json::json!({"kind": "curtain", "at": at}),
        Line::Over { at, json } => serde_json::json!({"kind": "over", "at": at, "json": json}),
        Line::Stop(stop) => {
            let mut value = serde_json::to_value(stop).unwrap_or_default();
            if let Some(map) = value.as_object_mut() {
                map.insert("kind".into(), "stop".into());
            }
            value
        }
    };
    let mut out = serde_json::to_vec(&value).unwrap_or_default();
    out.push(b'\n');
    out
}
