//! Resource containment at an owned process boundary, independent of game rules.
//!
//! This crate neither recognizes loops nor chooses a game result. The parent
//! supplies monotonic elapsed time and resident memory, then kills and reaps its
//! own child on a technical failure. A human thinking at an idle engine has no
//! work deadline. No clock or resource meter enters the rules kernel.
#![forbid(unsafe_code)]

use std::{fmt, io, io::Write, time::Duration};

pub mod frame;

/// Set by the process owner, never by a player or gateway command.
pub const REPORT_ENV: &str = "BAYLEE_WORK_REPORT";
/// One complete outer operation is starting.
pub const BUSY: u8 = 1;
/// Initialization or one complete outer operation has finished.
pub const IDLE: u8 = 2;
/// Private liveness marker. It never starts, completes or renews a work deadline.
pub const ALIVE: u8 = 3;

/// Watch ownership through writes to the parent's private stderr progress pipe.
/// Useful when stdin carries streamed work and cannot also be a liveness reader.
/// A missing owner exits the worker even during a stuck synchronous rules action.
/// Only use in a dedicated worker process with exclusively framed private stderr.
/// # Errors
/// The liveness thread cannot be created.
pub fn watch_progress_owner() -> io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("baylee-progress-liveness".into())
        .stack_size(64 * 1024)
        .spawn(|| {
            loop {
                std::thread::sleep(Duration::from_millis(250));
                if io::stderr().write_all(&[ALIVE]).is_err() {
                    std::process::exit(1);
                }
            }
        })
}

/// Watches the owner's otherwise empty stdin pipe on a separate thread.
/// Losing the owner terminates even a rules operation that never returns.
/// The thread lives until the worker process exits; it owns no game state.
///
/// # Errors
/// The liveness thread cannot be created.
pub fn watch_owner() -> io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("baylee-owner-liveness".into())
        .stack_size(64 * 1024)
        .spawn(|| {
            use io::Read as _;
            let mut pipe = io::stdin().lock();
            loop {
                match pipe.read(&mut [0_u8]) {
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    // This pipe carries no commands: bytes, EOF and errors
                    // all mean the ownership contract ended. No rules result.
                    _ => std::process::exit(1),
                }
            }
        })
}

/// The execution envelope; an exceeded bound is a technical interruption.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Maximum time to announce readiness after spawning.
    pub startup: Duration,
    /// Maximum time in one unresponsive operation, excluding human thinking.
    pub operation: Duration,
    /// Resident bytes, measured by the parent for the owned PID only.
    pub resident_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            startup: Duration::from_secs(30),
            operation: Duration::from_secs(60),
            resident_bytes: 2 * 1024 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Phase {
    Starting(Duration),
    Idle,
    Working(Duration),
}

/// Why an owned worker must stop. None of these means a player won or lost.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The child did not finish starting.
    Startup,
    /// One operation stopped responding.
    Operation,
    /// The resident-memory bound was reached.
    Memory,
    /// The private progress stream did not follow its protocol.
    Progress,
    /// The owner cannot establish process resource measurements.
    Monitor,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Startup => "engine initialization exceeded its time budget",
            Self::Operation => "engine operation exceeded its time budget",
            Self::Memory => "engine exceeded its resident-memory budget",
            Self::Progress => "engine progress stream was invalid",
            Self::Monitor => "engine resource monitoring was unavailable",
        })
    }
}

impl std::error::Error for Failure {}

/// A parent's monotonic view of one owned child's resource budget.
#[derive(Debug)]
pub struct Budget {
    limits: Limits,
    phase: Phase,
}

impl Budget {
    /// Starts the initialization deadline at the owner's elapsed time.
    #[must_use]
    pub const fn new(limits: Limits, now: Duration) -> Self {
        Self {
            limits,
            phase: Phase::Starting(now),
        }
    }

    /// Consumes a private pipe marker. Repeated Busy cannot renew a deadline.
    ///
    /// # Errors
    /// Unknown or out-of-order markers, or an operation already over its deadline.
    pub fn observe(&mut self, marker: u8, now: Duration) -> Result<(), Failure> {
        self.check(now, 0)?;
        self.phase = match (self.phase, marker) {
            (phase, ALIVE) => phase,
            (Phase::Idle, BUSY) => Phase::Working(now),
            (Phase::Starting(_) | Phase::Working(_), IDLE) => Phase::Idle,
            _ => return Err(Failure::Progress),
        };
        Ok(())
    }

    /// Checks sampled resources. The caller must kill and reap on failure.
    ///
    /// # Errors
    /// Resident memory or an active deadline reached its limit.
    pub fn check(&self, now: Duration, resident_bytes: u64) -> Result<(), Failure> {
        if resident_bytes >= self.limits.resident_bytes {
            return Err(Failure::Memory);
        }
        match self.phase {
            Phase::Starting(since) if now.saturating_sub(since) >= self.limits.startup => {
                Err(Failure::Startup)
            }
            Phase::Working(since) if now.saturating_sub(since) >= self.limits.operation => {
                Err(Failure::Operation)
            }
            _ => Ok(()),
        }
    }
}

/// Emits private progress only around outer host operations. A disabled
/// reporter has no I/O; neither path asks the rules to inspect repetition.
pub struct Reporter<W: Write>(Option<W>);

impl<W: Write> Reporter<W> {
    /// Announces readiness when a private pipe was provided.
    ///
    /// # Errors
    /// The owner's pipe is gone or cannot be written.
    pub fn new(writer: Option<W>) -> io::Result<Self> {
        let mut reporter = Self(writer);
        reporter.mark(IDLE)?;
        Ok(reporter)
    }

    /// Runs one host operation between Busy and Idle.
    ///
    /// # Errors
    /// A progress write fails; a failed initial write does not run the operation.
    pub fn work<T>(&mut self, operation: impl FnOnce() -> T) -> io::Result<T> {
        self.mark(BUSY)?;
        let value = operation();
        self.mark(IDLE)?;
        Ok(value)
    }

    fn mark(&mut self, byte: u8) -> io::Result<()> {
        if let Some(writer) = &mut self.0 {
            writer.write_all(&[byte])?;
            writer.flush()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
