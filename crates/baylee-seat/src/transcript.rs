//! What the bridge did, written down: one JSON line per event.
//!
//! Every question the seat was asked and who answered it (the standing
//! orders, the mind, the house), every refusal and every fallback, so "why
//! did it not answer my Lightning Bolt" has an answer after the game. The
//! lines are made by the seat core, which never holds the seat token or the
//! session token, so neither can reach a transcript.

use crate::RefusedBy;
use crate::seat::By;
use crate::wake::{Standing, Why};
use baylee_engine::choice::PlayerAction;
use serde::Serialize;
use std::io::Write as _;
use std::time::Instant;

/// One event at the seat.
#[derive(Clone, Debug, Serialize)]
pub struct Note {
    /// The question it concerns (0 for events that concern none).
    pub question: u64,
    /// The frame sequence number the seat had reached.
    pub seq: u64,
    /// The turn.
    pub turn: u32,
    /// What happened.
    #[serde(flatten)]
    pub event: Event,
}

/// What happened at the seat.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    /// The standing orders answered a question.
    Standing {
        /// The question's kind.
        kind: &'static str,
        /// Which order.
        why: Standing,
        /// The answer.
        action: PlayerAction,
    },
    /// The mind was asked.
    Asked {
        /// The question's kind.
        kind: &'static str,
        /// Why the standing orders did not answer it; `None` for a
        /// payment's next step.
        why: Option<Why>,
        /// The step it was asked in.
        step: baylee_view::Step,
        /// Whether it was asked on another seat's turn.
        their_turn: bool,
        /// A payment's next step.
        continuing: bool,
        /// A second asking, after a refusal.
        retry: bool,
        /// How long the bridge will wait, in milliseconds.
        budget_ms: u64,
        /// How many log lines went with it.
        log_lines: usize,
    },
    /// An answer was sent.
    Answered {
        /// Who made it.
        by: By,
        /// The answer.
        action: PlayerAction,
        /// How long the model took, when a mind made it.
        model_ms: Option<u64>,
        /// What the mind wrote beside it.
        note: Option<String>,
    },
    /// An answer was refused.
    Refused {
        /// By the bridge or by the table.
        by: RefusedBy,
        /// The answer.
        action: PlayerAction,
        /// Why.
        reason: String,
    },
    /// The mind did not answer.
    MindFailed {
        /// Why, in its words.
        error: String,
    },
    /// The mind's budget ran out.
    Expired,
    /// Nothing the bridge could send was taken; the seat leaves (the next
    /// note says so).
    Unanswerable,
    /// An answer came for a question that is gone.
    Late,
    /// The table moved on while the mind was thinking: the question it was
    /// thinking about is gone, and its answer will not be sent.
    Withdrawn,
    /// A question was sent again before the answer to it landed.
    Resent,
    /// The table said something in words.
    TableSaid {
        /// What.
        message: String,
    },
    /// The mind failed too often in a row and was taken off the table.
    MindDown,
    /// The bridge left the table.
    Left {
        /// Why.
        reason: String,
    },
    /// The game ended.
    Over {
        /// How, as the engine said it.
        result: String,
    },
    /// The room closed without showing the seat how the game ended: the
    /// table could not start, or its engine was lost.
    Closed,
}

/// Something handed every note as it is written.
struct Echo(Box<dyn FnMut(&Note) + Send>);

impl std::fmt::Debug for Echo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Echo")
    }
}

/// Where the notes go.
#[derive(Debug)]
pub struct Transcript {
    out: Option<std::io::BufWriter<std::fs::File>>,
    kept: Option<Vec<String>>,
    started: Instant,
    echo: Option<Echo>,
}

impl Transcript {
    /// Notes go nowhere.
    #[must_use]
    pub fn none() -> Self {
        Self {
            out: None,
            kept: None,
            started: Instant::now(),
            echo: None,
        }
    }

    /// Notes are kept in memory ([`Transcript::lines`]), for tests.
    #[must_use]
    pub fn memory() -> Self {
        Self {
            out: None,
            kept: Some(Vec::new()),
            started: Instant::now(),
            echo: None,
        }
    }

    /// Notes are written to a file, created or appended to.
    ///
    /// # Errors
    /// When the file cannot be opened.
    pub fn file(path: &std::path::Path) -> std::io::Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            out: Some(std::io::BufWriter::new(file)),
            kept: None,
            started: Instant::now(),
            echo: None,
        })
    }

    /// The same transcript, also handing every note to `echo` as it is
    /// written: the terminal's show ([`crate::show`]).
    #[must_use]
    pub fn echo(mut self, echo: impl FnMut(&Note) + Send + 'static) -> Self {
        self.echo = Some(Echo(Box::new(echo)));
        self
    }

    /// Writes one note, stamped with the milliseconds since the bridge
    /// started.
    pub fn write(&mut self, note: &Note) {
        if let Some(echo) = self.echo.as_mut() {
            (echo.0)(note);
        }
        self.line(&Line {
            at_ms: self.elapsed_ms(),
            note,
        });
    }

    /// Writes any serializable line, stamped the same way (the summary at
    /// the end of a game).
    pub fn write_value(&mut self, value: &impl Serialize) {
        #[derive(Serialize)]
        struct Stamped<'a, T: Serialize> {
            at_ms: u64,
            #[serde(flatten)]
            value: &'a T,
        }
        self.line(&Stamped {
            at_ms: self.elapsed_ms(),
            value,
        });
    }

    /// The lines kept in memory.
    #[must_use]
    pub fn lines(&self) -> &[String] {
        self.kept.as_deref().unwrap_or_default()
    }

    /// Flushes a file transcript.
    pub fn flush(&mut self) {
        if let Some(out) = self.out.as_mut() {
            let _ = out.flush();
        }
    }

    fn elapsed_ms(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn line(&mut self, value: &impl Serialize) {
        let Ok(text) = serde_json::to_string(value) else {
            return;
        };
        if let Some(kept) = self.kept.as_mut() {
            kept.push(text.clone());
        }
        if let Some(out) = self.out.as_mut() {
            // A transcript that cannot be written is not a reason to leave
            // the table.
            let _ = writeln!(out, "{text}");
        }
    }
}

#[derive(Serialize)]
struct Line<'a> {
    at_ms: u64,
    #[serde(flatten)]
    note: &'a Note,
}
