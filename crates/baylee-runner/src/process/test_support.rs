//! Fixtures shared by the process and runner tests.
use super::*;
use std::fs::File;
use std::sync::{Arc, Mutex};

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
    /// Request cancellation after this many process spawns.
    pub cancel_after_spawns: Option<usize>,
    /// Boundaries handed out, in order, to jobs on other threads.
    pub workers: std::collections::VecDeque<Arc<Mutex<MockOperations>>>,
    /// Every command's program and arguments, in the order they ran, shared
    /// between the boundaries of one test.
    pub journal: Option<Arc<Mutex<Vec<String>>>>,
    /// Commands whose arguments contain the word take this long, for real.
    pub slow: Vec<(String, Duration)>,
}

/// One mock boundary shared with the test that inspects it afterwards.
pub struct SharedMock(pub Arc<Mutex<MockOperations>>);

impl Operations for SharedMock {
    fn cancelled(&self) -> bool {
        self.0.lock().unwrap().cancelled()
    }
    fn output(&mut self, spec: Spec) -> Result<Output> {
        self.0.lock().unwrap().output(spec)
    }
    fn spawn(&mut self, spec: Spec) -> Result<Box<dyn Process>> {
        self.0.lock().unwrap().spawn(spec)
    }
    fn sleep(&mut self, duration: Duration) {
        self.0.lock().unwrap().sleep(duration);
    }
}

impl MockOperations {
    /// Takes a slow command's time first and then writes it down, so the
    /// journal's order is the order commands *finished* in.
    fn record(&self, spec: &Spec) {
        for (word, delay) in &self.slow {
            if spec.args.iter().any(|arg| arg == word) {
                std::thread::sleep(*delay);
            }
        }
        if let Some(journal) = &self.journal {
            let line = format!("{} {}", spec.program, spec.args.join(" "));
            journal.lock().unwrap().push(line);
        }
    }

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
    fn cancelled(&self) -> bool {
        self.cancel_after_spawns == Some(0)
    }
    fn output(&mut self, spec: Spec) -> Result<Output> {
        self.record(&spec);
        let (expected, output) = self.outputs.pop_front().expect("unexpected output call");
        assert_eq!(spec, expected);
        Ok(output)
    }

    fn spawn(&mut self, spec: Spec) -> Result<Box<dyn Process>> {
        self.record(&spec);
        let (expected, process) = self.processes.pop_front().expect("unexpected spawn call");
        assert_eq!(spec, expected);
        if let Some(path) = &spec.log {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create mock log directory");
            }
            File::create(path).expect("create mock log");
        }
        self.cancel_after_spawns = self
            .cancel_after_spawns
            .map(|count| count.saturating_sub(1));
        Ok(process)
    }

    fn sleep(&mut self, duration: Duration) {
        self.sleeps.push(duration);
    }

    fn worker(&mut self) -> Option<Box<dyn Operations + Send>> {
        let worker = self.workers.pop_front()?;
        Some(Box::new(SharedMock(worker)))
    }
}
