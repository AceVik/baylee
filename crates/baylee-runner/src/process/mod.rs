//! External processes, their timeouts, and a mockable boundary for tests.

use anyhow::{Result, bail};
use std::fmt;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Output};
use std::thread;
use std::time::Duration;

mod real;

const DEFAULT_PATH: &str = "/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin";

/// One external command, without a shell or an OS-specific argument builder.
#[derive(Clone, PartialEq, Eq)]
pub struct Spec {
    /// Executable name or path.
    pub program: String,
    /// Arguments, in their final order.
    pub args: Vec<String>,
    /// Environment additions. Debug output never includes their values.
    pub env: Vec<(String, String)>,
    /// Bytes handed to the child's stdin and then closed.
    pub stdin: Option<Vec<u8>>,
    /// File receiving both output streams for a spawned child.
    pub log: Option<PathBuf>,
    /// Prefix forwarded output lines with, for example, a VM name.
    pub prefix: Option<String>,
    /// Hard deadline for the whole command.
    pub timeout: Duration,
    /// Cleanup commands must still run after a shutdown request.
    pub cleanup: bool,
}

impl Spec {
    /// Starts a command with no additions and a one-minute deadline.
    pub fn new<I, S>(program: &str, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            program: program.to_owned(),
            args: args.into_iter().map(Into::into).collect(),
            env: Vec::new(),
            stdin: None,
            log: None,
            prefix: None,
            timeout: Duration::from_secs(60),
            cleanup: false,
        }
    }

    /// Adds an environment value to the child only.
    #[must_use]
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.to_owned(), value.to_owned()));
        self
    }

    /// Supplies the child's stdin and closes it after writing.
    #[must_use]
    pub fn with_stdin(mut self, bytes: Vec<u8>) -> Self {
        self.stdin = Some(bytes);
        self
    }

    /// Writes both output streams to one file.
    #[must_use]
    pub fn with_log(mut self, path: PathBuf) -> Self {
        self.log = Some(path);
        self
    }

    /// Prefixes every forwarded output line.
    #[must_use]
    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix);
        self
    }

    /// Replaces the hard deadline.
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Allows a cleanup command to finish after a shutdown signal.
    #[must_use]
    pub const fn for_cleanup(mut self) -> Self {
        self.cleanup = true;
        self
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.args).env("PATH", DEFAULT_PATH);
        command.envs(self.env.iter().map(|(key, value)| (key, value)));
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        command
    }
}

impl fmt::Debug for Spec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let env_keys: Vec<&str> = self.env.iter().map(|(key, _)| key.as_str()).collect();
        f.debug_struct("Spec")
            .field("program", &self.program)
            .field("args", &self.args)
            .field("env_keys", &env_keys)
            .field("log", &self.log)
            .field("prefix", &self.prefix)
            .field("timeout", &self.timeout)
            .field("cleanup", &self.cleanup)
            .finish_non_exhaustive()
    }
}

/// A running child. Every wait is bounded, and dropping it never leaves it running.
pub trait Process: Send {
    /// Returns a finished status once, or `None` while the child is running.
    ///
    /// # Errors
    /// Returns an error when the operating system cannot report the child status.
    fn try_wait(&mut self) -> Result<Option<ExitStatus>>;
    /// Waits up to the deadline, killing the child on timeout.
    ///
    /// # Errors
    /// Returns an error on process failure or when the deadline expires.
    fn wait(&mut self, timeout: Duration) -> Result<ExitStatus>;
    /// Requests termination and does not wait for it.
    ///
    /// # Errors
    /// Returns an error when termination cannot be requested.
    fn kill(&mut self) -> Result<()>;
    /// Requests graceful termination of a process group.
    ///
    /// # Errors
    /// Returns an error when termination cannot be requested.
    fn terminate(&mut self) -> Result<()> {
        self.kill()
    }
}

/// The process boundary used by GitHub, Tart, and the installer.
pub trait Operations {
    /// Whether shutdown was requested; mocks may override this.
    fn cancelled(&self) -> bool {
        false
    }
    /// Runs a command to completion and captures both output streams.
    ///
    /// # Errors
    /// Returns an error on spawn, I/O, or timeout failure.
    fn output(&mut self, spec: Spec) -> Result<Output>;
    /// Starts a child with optional log streaming.
    ///
    /// # Errors
    /// Returns an error when the child cannot be started or its streams configured.
    fn spawn(&mut self, spec: Spec) -> Result<Box<dyn Process>>;
    /// Sleeps without imposing a real delay on mocked tests.
    fn sleep(&mut self, duration: Duration);
    /// An independent boundary for a job on another thread, or `None` when
    /// jobs run one after the other on this one.
    fn worker(&mut self) -> Option<Box<dyn Operations + Send>> {
        None
    }
}

/// The production implementation using `std::process`.
#[derive(Default)]
pub struct RealOperations;

impl Operations for RealOperations {
    fn cancelled(&self) -> bool {
        crate::shutdown::requested()
    }
    fn output(&mut self, spec: Spec) -> Result<Output> {
        real::output(&spec)
    }

    fn spawn(&mut self, spec: Spec) -> Result<Box<dyn Process>> {
        real::spawn(&spec).map(|child| Box::new(child) as Box<dyn Process>)
    }

    fn sleep(&mut self, duration: Duration) {
        let started = std::time::Instant::now();
        while !self.cancelled() && started.elapsed() < duration {
            thread::sleep(
                duration
                    .saturating_sub(started.elapsed())
                    .min(Duration::from_millis(100)),
            );
        }
    }

    fn worker(&mut self) -> Option<Box<dyn Operations + Send>> {
        Some(Box::new(Self))
    }
}

/// Requires success and includes only stderr in the diagnostic, never stdin.
///
/// # Errors
/// Returns an error when the process exits unsuccessfully.
pub fn require_success(output: &Output, program: &str) -> Result<()> {
    if output.status.success() {
        Ok(())
    } else {
        bail!(
            "{program} failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )
    }
}

#[cfg(test)]
pub mod test_support;

#[cfg(test)]
mod tests;
