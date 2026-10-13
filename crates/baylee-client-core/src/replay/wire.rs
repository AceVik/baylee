//! The tape's lines as they are written (§4): `baylee_protocol::tape`, the
//! one definition the engine-server writes and this crate reads, under the
//! names this crate used before it landed, and the line shape the tape's
//! builder takes.

use serde::Deserialize;
use serde_json::value::RawValue;

use baylee_protocol::tape as proto;
pub use baylee_protocol::tape::{
    Basis as DivergedBy, Check, Divergence as Diverged, Header, Perspective as TapePerspective,
    Source, Stop as StopLine, StopKind as StopReason, TAPE_VERSION,
};

/// What a tape without its `stop` line says: cut in transit.
#[must_use]
pub const fn cut() -> StopLine {
    StopLine::bare(StopReason::Cut)
}

/// The perspective a header names, from its seat.
#[must_use]
pub const fn perspective_of(header: &Header) -> super::Perspective {
    let seat = baylee_core::ids::PlayerId::new(header.seat);
    match header.perspective {
        TapePerspective::Seat => super::Perspective::Seat(seat),
        TapePerspective::Public => super::Perspective::Public,
        TapePerspective::AllHands => super::Perspective::AllHands(seat),
    }
}

/// A perspective as a header spells it.
#[must_use]
pub const fn tape_perspective(perspective: super::Perspective) -> TapePerspective {
    match perspective {
        super::Perspective::Seat(_) => TapePerspective::Seat,
        super::Perspective::Public => TapePerspective::Public,
        super::Perspective::AllHands(_) => TapePerspective::AllHands,
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

/// Reads the header line. `None`: not a tape's header (of this version).
pub(super) fn parse_header(line: &[u8]) -> Option<Header> {
    match proto::parse_line(std::str::from_utf8(line).ok()?).ok()? {
        proto::Line::Header(header) => Some(header),
        _ => None,
    }
}

/// The header's versions, read without the rest, so another tape version is
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

/// Reads one line after the header. `Err(())`: not a line this format has
/// (a second header among them too).
pub(super) fn parse_line(line: &[u8]) -> Result<Line, ()> {
    let text = std::str::from_utf8(line).map_err(|_| ())?;
    Ok(match proto::parse_line(text).map_err(|_| ())? {
        proto::Line::Header(_) => return Err(()),
        proto::Line::Static { at, json } => Line::Static { at, json },
        proto::Line::View(view) => Line::View {
            at: view.at,
            n: view.n,
            check: view.check,
            seq: view.seq,
            json: view.json,
            log: view.log.filter(|raw| raw.get() != "null"),
        },
        proto::Line::Curtain { at } => Line::Curtain { at },
        proto::Line::Over { at, json } => Line::Over { at, json },
        proto::Line::Stop(stop) => Line::Stop(stop),
    })
}

/// Writes the header line, newline included.
pub(super) fn encode_header(header: &Header) -> Vec<u8> {
    let mut out = Vec::new();
    proto::write_line(&mut out, &proto::Line::Header(header.clone()));
    out
}

/// Writes one line, newline included.
pub(super) fn encode_line(line: &Line) -> Vec<u8> {
    let line = match line {
        Line::Static { at, json } => proto::Line::Static {
            at: *at,
            json: json.clone(),
        },
        Line::View {
            at,
            n,
            check,
            seq,
            json,
            log,
        } => proto::Line::View(proto::View {
            at: *at,
            n: *n,
            check: *check,
            seq: *seq,
            json: json.clone(),
            log: log.clone(),
        }),
        Line::Curtain { at } => proto::Line::Curtain { at: *at },
        Line::Over { at, json } => proto::Line::Over {
            at: *at,
            json: json.clone(),
        },
        Line::Stop(stop) => proto::Line::Stop(*stop),
    };
    let mut out = Vec::new();
    proto::write_line(&mut out, &line);
    out
}
