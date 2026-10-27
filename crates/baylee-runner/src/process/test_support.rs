//! Fixtures shared by the process and runner tests.
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
