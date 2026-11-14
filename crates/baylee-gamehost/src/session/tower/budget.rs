use super::{MAX_EVENTS, MAX_LINE_BYTES, MAX_RECORD_BYTES, MAX_RECORD_LINES, Refused};
use crate::session::{Session, SessionRegistry};
use baylee_engine::engine::LoopDetection;
use std::time::Instant;

/// Operator-selected native verification limits. These do not adjudicate games.
/// The live reservation byte limit must not exceed the allocated verifier budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayBudget {
    /// Total raw record bytes, at most 512 MiB.
    pub bytes: usize,
    /// Total JSON lines, including header/end, at most two million.
    pub lines: usize,
    /// Maximum raw JSON bytes in one line, excluding newline, at most one MiB.
    pub line_bytes: usize,
    /// Maximum engine journal entries, at most eight million.
    pub events: usize,
}

impl Default for ReplayBudget {
    fn default() -> Self {
        Self {
            bytes: 64 * 1024 * 1024,
            lines: 250_000,
            line_bytes: 1024 * 1024,
            events: 2_000_000,
        }
    }
}

impl ReplayBudget {
    pub(super) fn limits(self) -> Result<Limits, Refused> {
        if !(1..=512 * 1024 * 1024).contains(&self.bytes)
            || !(2..=2_000_000).contains(&self.lines)
            || !(1..=1024 * 1024).contains(&self.line_bytes)
            || !(1..=8_000_000).contains(&self.events)
        {
            return Err(Refused::Budget);
        }
        Ok(Limits {
            resources: self,
            turns: None,
        })
    }
}

#[derive(Clone, Copy)]
pub(super) struct Limits {
    pub resources: ReplayBudget,
    /// Legacy expedition compatibility only; native Tower has no artificial turn cap.
    pub turns: Option<(u32, u32)>,
}

impl Limits {
    pub(super) const fn legacy() -> Self {
        Self {
            resources: ReplayBudget {
                bytes: MAX_RECORD_BYTES,
                lines: MAX_RECORD_LINES,
                line_bytes: MAX_LINE_BYTES,
                events: MAX_EVENTS,
            },
            turns: Some((1, 500)),
        }
    }

    pub(super) fn check<D: LoopDetection, R: SessionRegistry>(
        self,
        session: &Session<D, R>,
        deadline: Instant,
    ) -> Result<(), Refused> {
        if Instant::now() >= deadline
            || session.engine.journal().len() > self.resources.events
            || self
                .turns
                .is_some_and(|(_, max)| session.engine.state().turn.number > max)
        {
            return Err(Refused::Budget);
        }
        Ok(())
    }
}
