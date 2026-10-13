//! A projection job (`docs/protocol.md` §"Replays"): instead of a game, an
//! engine the gateway started for a replay is handed a stored game record
//! ([`v1::ProjectRecord`]), plays it again on this build
//! ([`baylee_gamehost::replay::project`]) and sends back the tape one
//! perspective of it is sent, as [`v1::TapeChunk`]s, then exits.
//!
//! The record arrives on the engine link and goes nowhere else; what leaves
//! is the projection: the frames a seat (or a spectator) was sent, written as
//! [`baylee_protocol::tape`] lines, gzip members of [`TAPE_MEMBER_BYTES`]
//! each, the last ending on the tape's `stop` line.
//!
//! Synchronous and socket-free like [`crate::EngineRunner`], so the gateway's
//! end-to-end tests run it in-process; the binary calls it on a blocking
//! thread.

use std::io::{Read as _, Write as _};
use std::ops::ControlFlow;

use baylee_core::ids::PlayerId;
use baylee_gamehost::replay::{self, Moment, Perspective, Projection, Verify};
use baylee_protocol::tape::{self, Header, Line, Stop, StopKind};
use baylee_protocol::v1::{self, Envelope};
use flate2::Compression;
use flate2::read::MultiGzDecoder;
use flate2::write::GzEncoder;
use serde_json::value::RawValue;

/// How much of the tape, uncompressed, gathers before it is sent as one
/// [`v1::TapeChunk`] (one gzip member).
pub const TAPE_MEMBER_BYTES: usize = 32 * 1024;

/// The most a tape may take, compressed. Past it the projection stops and
/// the tape ends `too_large`.
pub const MAX_TAPE_BYTES: usize = 32 * 1024 * 1024;

/// The most a record may unpack to. The gateway refuses one over 8 MiB
/// compressed; this bounds what such a record may inflate to.
pub const MAX_RECORD_UNPACKED: u64 = 512 * 1024 * 1024;

/// Writes tape lines into gzip members and hands them on.
struct Writer<'a> {
    job: String,
    pending: Vec<u8>,
    seq: u32,
    sent: usize,
    full: bool,
    emit: &'a mut dyn FnMut(v1::TapeChunk),
}

impl Writer<'_> {
    fn line(&mut self, line: &Line) {
        tape::write_line(&mut self.pending, line);
        if self.pending.len() >= TAPE_MEMBER_BYTES {
            self.send(false);
        }
    }

    fn send(&mut self, last: bool) {
        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        // Writing into a `Vec` cannot fail.
        gz.write_all(&self.pending)
            .expect("compressing into memory");
        let data = gz.finish().expect("compressing into memory");
        self.pending.clear();
        self.sent = self.sent.saturating_add(data.len());
        if self.sent > MAX_TAPE_BYTES {
            self.full = true;
        }
        (self.emit)(v1::TapeChunk {
            job: self.job.clone(),
            seq: self.seq,
            data,
            last,
        });
        self.seq += 1;
    }
}

fn raw(bytes: &[u8]) -> Option<Box<RawValue>> {
    let text = std::str::from_utf8(bytes).ok()?;
    RawValue::from_string(text.to_owned()).ok()
}

/// The tape lines one moment's frames are.
fn lines_of(moment: &Moment) -> Vec<Line> {
    let mut out = Vec::new();
    for Envelope { msg } in &moment.frames {
        match msg {
            Some(v1::envelope::Msg::GameStatic(statics)) => {
                if let Some(json) = raw(&statics.static_json) {
                    out.push(Line::Static {
                        at: moment.at,
                        json,
                    });
                }
            }
            Some(v1::envelope::Msg::StateDelta(delta)) => {
                if let Some(json) = raw(&delta.view_json) {
                    out.push(Line::View(tape::View {
                        at: moment.at,
                        n: moment.input,
                        check: moment.check,
                        seq: delta.seq,
                        json,
                        log: if delta.log_json.is_empty() {
                            None
                        } else {
                            raw(&delta.log_json)
                        },
                    }));
                }
            }
            // The one question a projection hands on: the game's end.
            Some(v1::envelope::Msg::ChoiceRequest(choice)) => {
                if let Some(json) = raw(&choice.pending_json)
                    && json.get().starts_with(r#"{"GameOver""#)
                {
                    out.push(Line::Over {
                        at: moment.at,
                        json,
                    });
                }
            }
            Some(v1::envelope::Msg::Curtain(_)) => out.push(Line::Curtain { at: moment.at }),
            _ => {}
        }
    }
    out
}

/// The build that recorded `record`, as its header says.
fn recorded_by(record: &[u8]) -> Option<String> {
    let first = record.split(|&b| b == b'\n').next()?;
    let header: serde_json::Value = serde_json::from_slice(first).ok()?;
    header.get("build")?.as_str().map(str::to_owned)
}

fn stop_of(projection: &Projection, full: bool) -> Stop {
    let stop = match projection.stop {
        _ if full => StopKind::TooLarge,
        replay::Stop::End => StopKind::End,
        replay::Stop::Cut => StopKind::Cut,
        replay::Stop::Diverged { .. } => StopKind::Diverged,
        replay::Stop::Refused { .. } => StopKind::Refused,
        replay::Stop::Cancelled => StopKind::Cancelled,
    };
    Stop {
        stop,
        inputs: projection.inputs,
        exact_to: projection.exact_to,
        seen_to: projection.seen_to,
        diverged: projection.diverged,
    }
}

/// Runs one projection job: inflates the record, plays it again for the
/// order's perspective, and hands `emit` the tape's pieces in order, the last
/// with `last` set. Always ends on a `stop` line: a record that cannot be
/// read or a perspective that cannot be had is a tape that stops `refused`
/// before any frame.
pub fn run(order: &v1::ProjectRecord, build: &str, emit: &mut dyn FnMut(v1::TapeChunk)) -> Stop {
    let mut writer = Writer {
        job: order.job.clone(),
        pending: Vec::new(),
        seq: 0,
        sent: 0,
        full: false,
        emit,
    };
    let mut record = Vec::new();
    let inflated = MultiGzDecoder::new(order.record.as_slice())
        .take(MAX_RECORD_UNPACKED + 1)
        .read_to_end(&mut record);
    let perspective = match v1::project_record::Perspective::try_from(order.perspective) {
        Ok(v1::project_record::Perspective::Seat) => u8::try_from(order.seat)
            .ok()
            .map(|seat| Perspective::Seat(PlayerId::new(seat))),
        Ok(v1::project_record::Perspective::Public) => Some(Perspective::Public),
        _ => None,
    };
    let verify = match v1::project_record::Verify::try_from(order.verify) {
        Ok(v1::project_record::Verify::Note) => Verify::Note,
        _ => Verify::Stop,
    };
    let recorded_by = recorded_by(&record)
        .or_else(|| (!order.recorded_by.is_empty()).then(|| order.recorded_by.clone()))
        .unwrap_or_default();
    writer.line(&Line::Header(Header {
        tape: tape::TAPE_VERSION,
        view: baylee_view::VIEW_VERSION,
        build: build.to_owned(),
        source: tape::Source::Hosted,
        perspective: match perspective {
            Some(Perspective::Public) => tape::Perspective::Public,
            _ => tape::Perspective::Seat,
        },
        seat: match perspective {
            Some(Perspective::Seat(seat) | Perspective::AllHands(seat)) => seat.get(),
            _ => 0,
        },
        game: order.game_id.clone(),
        recorded_by,
    }));
    let usable = inflated.is_ok() && (record.len() as u64) <= MAX_RECORD_UNPACKED;
    let stop = match perspective.filter(|_| usable) {
        None => Stop::bare(StopKind::Refused),
        Some(perspective) => {
            let names = order.names.clone();
            let projected = replay::project(&record, names, perspective, verify, |moment| {
                for line in lines_of(&moment) {
                    writer.line(&line);
                }
                if writer.full {
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            });
            match projected {
                Ok(projection) => stop_of(&projection, writer.full),
                // The record's end where the replayed game has none, or a
                // line past the frames already sent that does not parse:
                // the tape stands up to there.
                Err(
                    replay::ReplayError::NotOver { .. } | replay::ReplayError::Unreadable { .. },
                ) => Stop::bare(StopKind::Cut),
                Err(_) => Stop::bare(StopKind::Refused),
            }
        }
    };
    writer.line(&Line::Stop(stop));
    writer.send(true);
    stop
}
