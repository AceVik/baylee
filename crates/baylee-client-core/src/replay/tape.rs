//! A tape in memory (§4, §6): its header, one [`Frame`] per distinct view,
//! the whole log decoded once, the statics, the end.
//!
//! Two doors, one shape: [`Tape::read`] takes the gzip a gateway sent (or a
//! tape file), [`Tape::from_moments`] the moments a local projection handed
//! its sink. Both feed the same lines through one builder, so a local replay
//! and a hosted one cannot differ in how they are read.

use baylee_engine::choice::Pending;
use baylee_protocol::v1::{self, Envelope};
use baylee_view::{GameStatic, LogEntry, LogTail, PlayerView, SpectatorView};
use serde::Deserialize;
use serde_json::value::RawValue;

use super::wire::{self, Check, Header, Line, StopLine};

/// A payload as the tape wrote it.
#[derive(Clone, Debug)]
struct Raw(Box<RawValue>);

impl PartialEq for Raw {
    fn eq(&self, other: &Self) -> bool {
        self.0.get() == other.0.get()
    }
}

impl Raw {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(bytes).ok()?;
        RawValue::from_string(text.to_owned()).ok().map(Self)
    }
}

/// Why a tape cannot be played.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TapeRefusal {
    /// Not a tape at all (no header line).
    NotATape,
    /// A tape of another format version.
    OtherTape(u32),
    /// A tape whose payloads are another view version than this client
    /// draws: refused as a live table of another version is.
    OtherView {
        /// The tape's.
        tape: u32,
        /// This client's.
        ours: u32,
    },
    /// Line `line` (1-based) is not one this format has.
    Unreadable {
        /// Its number.
        line: usize,
    },
    /// A payload did not decode as the view it claims to be.
    Payload(String),
}

impl std::fmt::Display for TapeRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotATape => f.write_str("this file is not a replay"),
            Self::OtherTape(v) => write!(
                f,
                "this replay is format {v}, this client reads {}",
                wire::TAPE_VERSION
            ),
            // `host_message`'s words for a live table of another version.
            Self::OtherView { tape, ours } => write!(
                f,
                "this client renders game views version {ours}, the table speaks {tape}"
            ),
            Self::Unreadable { line } => write!(f, "replay line {line} is unreadable"),
            Self::Payload(why) => write!(f, "unreadable game state: {why}"),
        }
    }
}

/// Where a step stands in the game, read off its view without the rest.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
pub struct Place {
    /// The turn.
    pub turn: u32,
    /// The phase.
    pub phase: baylee_view::Phase,
    /// The step.
    pub step: baylee_view::Step,
    /// The active seat.
    pub active: baylee_core::ids::PlayerId,
}

/// One step of a replay: a view the perspective was shown.
#[derive(Clone, PartialEq, Debug)]
pub struct Frame {
    /// The record's host time when it was made.
    pub at: u64,
    /// The record input after which it was made (`None`: before the first).
    pub n: Option<u64>,
    /// How far it is known to be the recorded game's.
    pub check: Check,
    /// The view's own `seq` as projected.
    pub seq: u64,
    /// Where the game stands.
    pub place: Place,
    /// How many log lines the perspective held once this frame was shown.
    pub log_len: usize,
    /// The newest static at or before this frame (an index into the
    /// tape's statics), if any came yet.
    pub statics: Option<usize>,
    view: Raw,
}

/// What a local projection hands its sink, in this crate's words:
/// `baylee_gamehost::replay::Moment`, which this crate cannot link.
#[derive(Clone, Debug)]
pub struct TapeMoment {
    /// The record input after which it was made (`None`: before the first).
    pub input: Option<u64>,
    /// The record's host time.
    pub at: u64,
    /// How far it is known to be the recorded game's.
    pub check: Check,
    /// The envelopes the perspective was sent.
    pub frames: Vec<Envelope>,
}

/// A replay's tape, read.
#[derive(Clone, PartialEq, Debug)]
pub struct Tape {
    /// The first line.
    pub header: Header,
    /// The steps, one per distinct view.
    pub frames: Vec<Frame>,
    /// When the table opened, and before which step (`None`: no curtain).
    pub curtain: Option<(u64, usize)>,
    /// The record's time at the game's end, when it ended.
    pub over_at: Option<u64>,
    /// How far it is the recorded game.
    pub stop: StopLine,
    statics: Vec<(u64, Raw)>,
    log: Vec<LogEntry>,
    over: Option<Raw>,
}

/// Builds a [`Tape`] line by line, the one way both doors share.
struct Builder {
    tape: Tape,
}

impl Builder {
    fn new(header: Header) -> Result<Self, TapeRefusal> {
        if header.tape != wire::TAPE_VERSION {
            return Err(TapeRefusal::OtherTape(header.tape));
        }
        if header.view != baylee_view::VIEW_VERSION {
            return Err(TapeRefusal::OtherView {
                tape: header.view,
                ours: baylee_view::VIEW_VERSION,
            });
        }
        Ok(Self {
            tape: Tape {
                header,
                frames: Vec::new(),
                curtain: None,
                over_at: None,
                stop: StopLine::cut(),
                statics: Vec::new(),
                log: Vec::new(),
                over: None,
            },
        })
    }

    fn push(&mut self, line: Line) -> Result<(), TapeRefusal> {
        let tape = &mut self.tape;
        match line {
            Line::Static { at, json } => tape.statics.push((at, Raw(json))),
            Line::View {
                at,
                n,
                check,
                seq,
                json,
                log,
            } => {
                if let Some(log) = log {
                    let tail: LogTail = serde_json::from_str(log.get())
                        .map_err(|e| TapeRefusal::Payload(e.to_string()))?;
                    append(&mut tape.log, tail);
                }
                let statics = tape.statics.len().checked_sub(1);
                // A frame repeating the last view and `seq` carries only the
                // next part of a long log (`LOG_TAIL_CAP`): one step.
                if let Some(last) = tape.frames.last_mut()
                    && last.seq == seq
                    && last.view.0.get() == json.get()
                {
                    last.log_len = tape.log.len();
                    last.statics = statics;
                    return Ok(());
                }
                let place: Place = serde_json::from_str(json.get())
                    .map_err(|e| TapeRefusal::Payload(e.to_string()))?;
                tape.frames.push(Frame {
                    at,
                    n,
                    check,
                    seq,
                    place,
                    log_len: tape.log.len(),
                    statics,
                    view: Raw(json),
                });
            }
            Line::Curtain { at } => tape.curtain = Some((at, tape.frames.len())),
            Line::Over { at, json } => {
                tape.over_at = Some(at);
                tape.over = Some(Raw(json));
            }
            Line::Stop(stop) => tape.stop = stop,
        }
        Ok(())
    }

    fn finish(self) -> Tape {
        self.tape
    }
}

/// Appends a tail to the log as a client does: by `from`. A tail that
/// starts inside what is held replaces from there; one that starts past it
/// is placed after what is held (nothing is invented for the gap).
fn append(log: &mut Vec<LogEntry>, tail: LogTail) {
    let from = tail.from as usize;
    if from < log.len() {
        log.truncate(from);
    }
    log.extend(tail.entries);
}

impl Tape {
    /// Reads a tape as a gateway sends it: gzip (one member or several) of
    /// JSON Lines. A tape cut short reads up to its last whole line and
    /// stops as `cut`.
    ///
    /// # Errors
    /// [`TapeRefusal`] for a file that is no tape, another tape or view
    /// version, or a line this format does not have.
    pub fn read(gzip: &[u8]) -> Result<Self, TapeRefusal> {
        let text = crate::bugreport::inflate(gzip).ok_or(TapeRefusal::NotATape)?;
        Self::read_lines(&text)
    }

    /// [`Tape::read`] of the JSON Lines themselves.
    ///
    /// # Errors
    /// As [`Tape::read`].
    pub fn read_lines(text: &[u8]) -> Result<Self, TapeRefusal> {
        let mut lines = text
            .split(|b| *b == b'\n')
            .filter(|line| !line.iter().all(u8::is_ascii_whitespace));
        let first = lines.next().ok_or(TapeRefusal::NotATape)?;
        let Some(header) = wire::parse_header(first) else {
            return Err(match wire::header_version(first) {
                Some((tape, _)) if tape != wire::TAPE_VERSION => TapeRefusal::OtherTape(tape),
                Some((_, Some(view))) if view != baylee_view::VIEW_VERSION => {
                    TapeRefusal::OtherView {
                        tape: view,
                        ours: baylee_view::VIEW_VERSION,
                    }
                }
                _ => TapeRefusal::NotATape,
            });
        };
        let mut builder = Builder::new(header)?;
        for (k, line) in lines.enumerate() {
            let line =
                wire::parse_line(line).map_err(|()| TapeRefusal::Unreadable { line: k + 2 })?;
            builder.push(line)?;
        }
        Ok(builder.finish())
    }

    /// Builds the tape a local projection's moments make, without writing
    /// it anywhere: the same thing [`Tape::read`] makes of their lines.
    ///
    /// # Errors
    /// [`TapeRefusal`] for another view version in a static, or a payload
    /// that is not what it claims.
    pub fn from_moments(
        header: Header,
        moments: impl IntoIterator<Item = TapeMoment>,
        stop: StopLine,
    ) -> Result<Self, TapeRefusal> {
        let mut builder = Builder::new(header)?;
        for moment in moments {
            for line in lines_of(&moment)? {
                builder.push(line)?;
            }
        }
        builder.push(Line::Stop(stop))?;
        Ok(builder.finish())
    }

    /// The tape as JSON Lines (not gzipped): what [`Tape::read_lines`]
    /// reads back as this tape. Split log tails are written merged.
    #[must_use]
    pub fn write_lines(&self) -> Vec<u8> {
        let mut out = wire::encode_header(&self.header);
        let mut sent_static = None;
        let mut sent_log = 0;
        for (i, frame) in self.frames.iter().enumerate() {
            if self.curtain.is_some_and(|(_, before)| before == i) {
                let at = self.curtain.map_or(0, |(at, _)| at);
                out.extend(wire::encode_line(&Line::Curtain { at }));
            }
            if let Some(to) = frame.statics
                && frame.statics != sent_static
            {
                // Every static up to this frame's, in order.
                let from = sent_static.map_or(0, |s| s + 1);
                for (at, json) in &self.statics[from..=to] {
                    out.extend(wire::encode_line(&Line::Static {
                        at: *at,
                        json: json.0.clone(),
                    }));
                }
                sent_static = frame.statics;
            }
            let log = (frame.log_len > sent_log).then(|| {
                let tail = LogTail {
                    from: sent_log as u32,
                    entries: self.log[sent_log..frame.log_len].to_vec(),
                };
                RawValue::from_string(serde_json::to_string(&tail).unwrap_or_default())
                    .unwrap_or_else(|_| null())
            });
            sent_log = frame.log_len;
            out.extend(wire::encode_line(&Line::View {
                at: frame.at,
                n: frame.n,
                check: frame.check,
                seq: frame.seq,
                json: frame.view.0.clone(),
                log,
            }));
        }
        if let Some((at, before)) = self.curtain
            && before >= self.frames.len()
        {
            out.extend(wire::encode_line(&Line::Curtain { at }));
        }
        if let (Some(at), Some(json)) = (self.over_at, &self.over) {
            out.extend(wire::encode_line(&Line::Over {
                at,
                json: json.0.clone(),
            }));
        }
        out.extend(wire::encode_line(&Line::Stop(self.stop)));
        out
    }

    /// The tape gzipped, as a gateway sends it.
    #[must_use]
    pub fn write(&self) -> Vec<u8> {
        crate::bugreport::gzip(&self.write_lines())
    }

    /// Whether this tape is the spectators' table.
    #[must_use]
    pub fn public(&self) -> bool {
        self.header.perspective == "public"
    }

    /// The view at step `i`, as the renderer reads a seat's: a public
    /// tape's `SpectatorView` drawn from seat 0, as a spectator socket's.
    ///
    /// # Errors
    /// The payload does not decode (`host_message`'s words).
    pub fn view(&self, i: usize) -> Result<PlayerView, String> {
        let frame = self.frames.get(i).ok_or("no such moment")?;
        let raw = frame.view.0.get();
        if self.public() {
            serde_json::from_str::<SpectatorView>(raw)
                .map(|view| view.into_player_view(baylee_core::ids::PlayerId::new(0)))
                .map_err(|e| format!("unreadable game state: {e}"))
        } else {
            serde_json::from_str::<PlayerView>(raw)
                .map_err(|e| format!("unreadable game state: {e}"))
        }
    }

    /// The static payload newest at or before step `i`.
    ///
    /// # Errors
    /// The payload does not decode.
    pub fn statics_at(&self, i: usize) -> Option<Result<GameStatic, String>> {
        let index = self.frames.get(i)?.statics?;
        Some(
            serde_json::from_str(self.statics[index].1.0.get())
                .map_err(|e| format!("unreadable game setup: {e}")),
        )
    }

    /// Whether a static arrived between step `i - 1` and step `i`.
    #[must_use]
    pub fn static_arrives_at(&self, i: usize) -> bool {
        let now = self.frames.get(i).and_then(|f| f.statics);
        let before = i
            .checked_sub(1)
            .and_then(|k| self.frames.get(k))
            .and_then(|f| f.statics);
        now.is_some() && now != before
    }

    /// The log lines step `i` adds to step `i - 1`'s, as a live tail.
    #[must_use]
    pub fn tail_at(&self, i: usize) -> Option<LogTail> {
        let to = self.frames.get(i)?.log_len;
        let from = i
            .checked_sub(1)
            .and_then(|k| self.frames.get(k))
            .map_or(0, |f| f.log_len);
        (to > from).then(|| LogTail {
            from: from as u32,
            entries: self.log[from..to].to_vec(),
        })
    }

    /// The whole log to step `i`, told from its first line (a jump's).
    #[must_use]
    pub fn log_to(&self, i: usize) -> LogTail {
        let to = self.frames.get(i).map_or(0, |f| f.log_len);
        LogTail {
            from: 0,
            entries: self.log[..to].to_vec(),
        }
    }

    /// The whole log as the tape holds it.
    #[must_use]
    pub fn log(&self) -> &[LogEntry] {
        &self.log
    }

    /// The `GameOver` question, when the tape reaches the end.
    ///
    /// # Errors
    /// The payload does not decode.
    pub fn over(&self) -> Option<Result<Pending, String>> {
        self.over.as_ref().map(|raw| {
            serde_json::from_str(raw.0.get()).map_err(|e| format!("unreadable choice: {e}"))
        })
    }

    /// The step whose input is the greatest at or before `n` (a perspective
    /// switch keeps the place this way: perspectives dedupe differently).
    #[must_use]
    pub fn step_at_input(&self, n: Option<u64>) -> usize {
        let Some(n) = n else {
            return 0;
        };
        self.frames
            .iter()
            .rposition(|f| f.n.is_none_or(|k| k <= n))
            .unwrap_or(0)
    }
}

fn null() -> Box<RawValue> {
    RawValue::from_string("null".to_owned()).unwrap_or_else(|_| unreachable!("null is JSON"))
}

/// The lines one moment's envelopes make, in order.
fn lines_of(moment: &TapeMoment) -> Result<Vec<Line>, TapeRefusal> {
    let bad = |what: &str| TapeRefusal::Payload(format!("{what} is not JSON"));
    let mut lines = Vec::new();
    for envelope in &moment.frames {
        let Some(msg) = &envelope.msg else { continue };
        match msg {
            v1::envelope::Msg::GameStatic(msg) => {
                if msg.view_version != baylee_view::VIEW_VERSION {
                    return Err(TapeRefusal::OtherView {
                        tape: msg.view_version,
                        ours: baylee_view::VIEW_VERSION,
                    });
                }
                lines.push(Line::Static {
                    at: moment.at,
                    json: Raw::from_bytes(&msg.static_json)
                        .ok_or_else(|| bad("a static"))?
                        .0,
                });
            }
            v1::envelope::Msg::StateDelta(delta) => lines.push(Line::View {
                at: moment.at,
                n: moment.input,
                check: moment.check,
                seq: delta.seq,
                json: Raw::from_bytes(&delta.view_json)
                    .ok_or_else(|| bad("a view"))?
                    .0,
                log: if delta.log_json.is_empty() {
                    None
                } else {
                    Some(
                        Raw::from_bytes(&delta.log_json)
                            .ok_or_else(|| bad("a log"))?
                            .0,
                    )
                },
            }),
            v1::envelope::Msg::Curtain(_) => lines.push(Line::Curtain { at: moment.at }),
            v1::envelope::Msg::ChoiceRequest(req) => {
                // Only the end is kept: a projection asks nobody anything.
                let over = serde_json::from_slice::<Pending>(&req.pending_json)
                    .is_ok_and(|p| matches!(p, Pending::GameOver(_)));
                if over {
                    lines.push(Line::Over {
                        at: moment.at,
                        json: Raw::from_bytes(&req.pending_json)
                            .ok_or_else(|| bad("the end"))?
                            .0,
                    });
                }
            }
            _ => {}
        }
    }
    Ok(lines)
}
