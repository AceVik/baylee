//! External processes, their timeouts, and a mockable boundary for tests.

use anyhow::{Context, Result, bail};
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

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

    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.args).env("PATH", DEFAULT_PATH);
        command.envs(self.env.iter().map(|(key, value)| (key, value)));
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
}

/// The process boundary used by GitHub, Tart, and the installer.
pub trait Operations {
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
}

/// The production implementation using `std::process`.
#[derive(Default)]
pub struct RealOperations;

struct RealProcess {
    child: Child,
    readers: Vec<JoinHandle<()>>,
}

impl RealProcess {
    fn join_readers(&mut self) {
        for reader in self.readers.drain(..) {
            let _ = reader.join();
        }
    }
}

impl Process for RealProcess {
    fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        let status = self.child.try_wait().context("checking child status")?;
        if status.is_some() {
            self.join_readers();
        }
        Ok(status)
    }

    fn wait(&mut self, timeout: Duration) -> Result<ExitStatus> {
        let started = Instant::now();
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(status);
            }
            if started.elapsed() >= timeout {
                self.kill()?;
                let _ = self.child.wait();
                self.join_readers();
                bail!("child timed out after {}s", timeout.as_secs());
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    fn kill(&mut self) -> Result<()> {
        self.child.kill().context("killing child")
    }
}

impl Drop for RealProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.join_readers();
    }
}

fn forward<R: Read + Send + 'static>(reader: R, prefix: String) -> JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(reader).lines().map_while(Result::ok) {
            println!("{prefix} {line}");
        }
    })
}

fn spawn_real(spec: &Spec) -> Result<Box<dyn Process>> {
    let mut command = spec.command();
    command.stdin(if spec.stdin.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    if let Some(path) = &spec.log {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let out = File::create(path).with_context(|| format!("creating {}", path.display()))?;
        let err = out.try_clone().context("cloning the log file")?;
        command.stdout(Stdio::from(out)).stderr(Stdio::from(err));
    } else if spec.prefix.is_some() {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
    } else {
        command.stdout(Stdio::null()).stderr(Stdio::null());
    }
    let mut child = command
        .spawn()
        .with_context(|| format!("spawning {}", spec.program))?;
    if let Some(bytes) = &spec.stdin {
        let mut input = child.stdin.take().context("opening child stdin")?;
        input.write_all(bytes).context("writing child stdin")?;
        drop(input);
    }
    let mut readers = Vec::new();
    if let Some(prefix) = &spec.prefix {
        if let Some(stdout) = child.stdout.take() {
            readers.push(forward(stdout, prefix.clone()));
        }
        if let Some(stderr) = child.stderr.take() {
            readers.push(forward(stderr, prefix.clone()));
        }
    }
    Ok(Box::new(RealProcess { child, readers }))
}

fn kill_process(id: u32) {
    let _ = Command::new("/bin/kill").arg(id.to_string()).status();
}

fn output_with_timeout(spec: &Spec) -> Result<Output> {
    let mut command = spec.command();
    command
        .stdin(if spec.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .with_context(|| format!("spawning {}", spec.program))?;
    if let Some(bytes) = &spec.stdin {
        let mut input = child.stdin.take().context("opening child stdin")?;
        input.write_all(bytes).context("writing child stdin")?;
        drop(input);
    }
    let id = child.id();
    let (sender, receiver) = mpsc::channel();
    let waiter = thread::spawn(move || {
        let result = child.wait_with_output();
        let _ = sender.send(result);
    });
    if let Ok(result) = receiver.recv_timeout(spec.timeout) {
        let output = result.context("reading child output")?;
        waiter
            .join()
            .map_err(|_| anyhow::anyhow!("output thread panicked"))?;
        Ok(output)
    } else {
        kill_process(id);
        let _ = waiter.join();
        bail!(
            "{} timed out after {}s",
            spec.program,
            spec.timeout.as_secs()
        );
    }
}

impl Operations for RealOperations {
    fn output(&mut self, spec: Spec) -> Result<Output> {
        output_with_timeout(&spec)
    }

    fn spawn(&mut self, spec: Spec) -> Result<Box<dyn Process>> {
        spawn_real(&spec)
    }

    fn sleep(&mut self, duration: Duration) {
        thread::sleep(duration);
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
/// Fixtures shared by the process and runner tests.
pub mod test_support {
    use super::*;

    /// A real exit status for this platform.
    pub fn status(code: i32) -> ExitStatus {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            ExitStatus::from_raw(code << 8)
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::ExitStatusExt;
            ExitStatus::from_raw(code as u32)
        }
    }

    /// An output fixture with empty stderr.
    pub fn output(code: i32, stdout: &str) -> Output {
        Output {
            status: status(code),
            stdout: stdout.as_bytes().to_vec(),
            stderr: Vec::new(),
        }
    }

    /// A child whose readiness is scripted.
    pub struct MockProcess {
        status: ExitStatus,
        ready: std::collections::VecDeque<bool>,
        /// Records whether termination was requested.
        pub killed: bool,
    }

    impl MockProcess {
        /// A child that finishes immediately with `status`.
        #[must_use]
        pub fn new(status: ExitStatus) -> Self {
            Self {
                status,
                ready: std::collections::VecDeque::new(),
                killed: false,
            }
        }

        /// A child that returns each readiness value in order, then finishes.
        #[must_use]
        pub fn with_steps(status: ExitStatus, steps: &[bool]) -> Self {
            Self {
                status,
                ready: steps.iter().copied().collect(),
                killed: false,
            }
        }
    }

    impl Process for MockProcess {
        fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
            if self.ready.pop_front().unwrap_or(true) {
                Ok(Some(self.status))
            } else {
                Ok(None)
            }
        }

        fn wait(&mut self, _timeout: Duration) -> Result<ExitStatus> {
            Ok(self.status)
        }

        fn kill(&mut self) -> Result<()> {
            self.killed = true;
            Ok(())
        }
    }

    /// The complete mock process boundary.
    #[derive(Default)]
    pub struct MockOperations {
        outputs: std::collections::VecDeque<(Spec, Output)>,
        processes: std::collections::VecDeque<(Spec, Box<dyn Process>)>,
        /// Delays requested by production logic, with no real sleeping.
        pub sleeps: Vec<Duration>,
    }

    impl MockOperations {
        /// Queues one completed command result.
        pub fn push_output(&mut self, spec: Spec, output: Output) {
            self.outputs.push_back((spec, output));
        }

        /// Queues one child result.
        pub fn push_process(&mut self, spec: Spec, process: Box<dyn Process>) {
            self.processes.push_back((spec, process));
        }

        /// Asserts that every queued command was consumed.
        ///
        /// # Panics
        /// Panics when a fixture was not consumed or an unexpected command arrives.
        pub fn assert_done(&self) {
            assert!(self.outputs.is_empty(), "unused output fixtures");
            assert!(self.processes.is_empty(), "unused child fixtures");
        }
    }

    impl Operations for MockOperations {
        fn output(&mut self, spec: Spec) -> Result<Output> {
            let (expected, output) = self.outputs.pop_front().expect("unexpected output call");
            assert_eq!(spec, expected);
            Ok(output)
        }

        fn spawn(&mut self, spec: Spec) -> Result<Box<dyn Process>> {
            let (expected, process) = self.processes.pop_front().expect("unexpected spawn call");
            assert_eq!(spec, expected);
            if let Some(path) = &spec.log {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).expect("create mock log directory");
                }
                File::create(path).expect("create mock log");
            }
            Ok(process)
        }

        fn sleep(&mut self, duration: Duration) {
            self.sleeps.push(duration);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{MockOperations, MockProcess, output, status};
    use super::*;

    #[test]
    fn debug_never_prints_environment_values_or_stdin() {
        let spec = Spec::new("gh", ["api"])
            .with_env("GH_TOKEN", "secret-value")
            .with_stdin(b"registration-secret".to_vec());
        let shown = format!("{spec:?}");
        assert!(!shown.contains("secret-value"));
        assert!(!shown.contains("registration-secret"));
        assert!(shown.contains("GH_TOKEN"));
    }

    #[test]
    fn output_timeout_kills_the_child() {
        let mut real = RealOperations;
        let error = real
            .output(Spec::new("/bin/sh", ["-c", "sleep 5"]).with_timeout(Duration::from_millis(30)))
            .expect_err("the child must be stopped");
        assert!(error.to_string().contains("timed out"), "{error}");
    }

    #[test]
    fn a_mock_child_can_finish_on_a_later_poll() {
        let mut process = MockProcess::with_steps(status(0), &[false, true]);
        assert_eq!(process.try_wait().unwrap(), None);
        assert_eq!(process.try_wait().unwrap(), Some(status(0)));
    }

    #[test]
    fn the_mock_boundary_checks_every_argument() {
        let spec = Spec::new("gh", ["api", "repos/example"]);
        let mut mock = MockOperations::default();
        mock.push_output(spec.clone(), output(0, "{}"));
        let got = mock.output(spec).unwrap();
        assert_eq!(got.stdout, b"{}");
        mock.assert_done();
    }
}
