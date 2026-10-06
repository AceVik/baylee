//! The seat bridge program (`baylee-seat`) beside this client, run as a
//! child: for a key kept in the OS credential store (`baylee-seat key`,
//! [`KeyRunner`]) and for a language model at a table this client hosts
//! (`baylee-seat join … --tethered`, [`Bridge`]).
//!
//! Desktop only (`crate::seatpanel::DESKTOP`): a browser and a phone have no
//! bridge beside them. Nothing here runs in a frame: each child is waited
//! on by a thread of its own, and a frame only takes what has come back.
//!
//! A key goes to the child on its stdin and nowhere else: not its
//! arguments, not its environment, not a log.

use baylee_client_core::llmseat::keys::{KeyEntry, KeyJob, KeyState};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

/// The variable that names the bridge program, over the one beside this
/// client.
pub(crate) const BIN_ENV: &str = "BAYLEE_SEAT_BIN";

/// The bridge program: the one `BAYLEE_SEAT_BIN` names, else `baylee-seat`
/// beside this client's own executable; `None` where there is neither.
pub(crate) fn program() -> Option<PathBuf> {
    if let Some(named) = std::env::var_os(BIN_ENV).filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(named));
    }
    let name = if cfg!(windows) {
        "baylee-seat.exe"
    } else {
        "baylee-seat"
    };
    let beside = std::env::current_exe().ok()?.parent()?.join(name);
    beside.is_file().then_some(beside)
}

/// What came back from the key jobs started, waiting to be taken.
type Answers = Arc<Mutex<Vec<(KeyEntry, Result<KeyState, String>)>>>;

/// Runs `baylee-seat key` jobs, one thread each, and hands back what each
/// came to.
#[derive(Default)]
pub(crate) struct KeyRunner {
    answers: Answers,
}

impl KeyRunner {
    /// Starts `job`.
    pub(crate) fn run(&self, job: KeyJob) {
        let answers = Arc::clone(&self.answers);
        std::thread::spawn(move || {
            let entry = job.entry().clone();
            let came = run_key(&job);
            answers
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push((entry, came));
        });
    }

    /// What has come back since last asked.
    pub(crate) fn take(&self) -> Vec<(KeyEntry, Result<KeyState, String>)> {
        std::mem::take(&mut *self.answers.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// Runs one key job to its end: the state the bridge prints, or the
/// sentence it failed with (its last line on stderr).
fn run_key(job: &KeyJob) -> Result<KeyState, String> {
    let program = program().ok_or_else(no_program)?;
    let mut child = Command::new(program)
        .args(job.args())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("the seat bridge did not start: {e}"))?;
    if let Some(mut stdin) = child.stdin.take()
        && let Some(line) = job.stdin()
    {
        // A failed write shows as the bridge's own refusal below.
        let _ = stdin.write_all(line.as_bytes());
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("the seat bridge did not answer: {e}"))?;
    let said = String::from_utf8_lossy(&out.stdout);
    if out.status.success() {
        return Ok(KeyState::parse(said.lines().last().unwrap_or_default()));
    }
    let why = String::from_utf8_lossy(&out.stderr);
    let why = why
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("the seat bridge refused")
        .trim_start_matches("Error: ");
    Err(baylee_client_core::llmseat::blank_key_shapes(why)
        .chars()
        .take(200)
        .collect())
}

/// Why no bridge can be run.
pub(crate) fn no_program() -> String {
    format!("no seat bridge (baylee-seat) beside this client, and {BIN_ENV} names none")
}

/// A bridge seated at a table by this client: its process, the stdin that
/// holds it (`--tethered`: closing it stops the bridge, and before the game
/// gives the chair up), and the last line it printed.
pub(crate) struct Bridge {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    last: Arc<Mutex<Option<String>>>,
    /// How many lines it has printed, so a page showing the last one is
    /// rebuilt when another comes, and only then.
    heard: Arc<AtomicUsize>,
}

impl Bridge {
    /// Starts `baylee-seat` with `args`, the room's password (if any) in its
    /// environment, never its arguments.
    pub(crate) fn start(args: &[String], password: Option<&str>) -> Result<Self, String> {
        let program = program().ok_or_else(no_program)?;
        let mut command = Command::new(program);
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(password) = password {
            command.env("BAYLEE_ROOM_PASSWORD", password);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("the seat bridge did not start: {e}"))?;
        let last = Arc::new(Mutex::new(None));
        let heard = Arc::new(AtomicUsize::new(0));
        let streams: [Option<Box<dyn std::io::Read + Send>>; 2] = [
            child.stdout.take().map(|s| Box::new(s) as _),
            child.stderr.take().map(|s| Box::new(s) as _),
        ];
        for stream in streams.into_iter().flatten() {
            let last = Arc::clone(&last);
            let heard = Arc::clone(&heard);
            std::thread::spawn(move || {
                for line in BufReader::new(stream).lines().map_while(Result::ok) {
                    let line = baylee_client_core::llmseat::blank_key_shapes(line.trim());
                    if !line.is_empty() {
                        *last.lock().unwrap_or_else(PoisonError::into_inner) =
                            Some(line.chars().take(240).collect());
                        heard.fetch_add(1, Ordering::Release);
                    }
                }
            });
        }
        Ok(Self {
            stdin: child.stdin.take(),
            child: Some(child),
            last,
            heard,
        })
    }

    /// Hands the bridge an order line (a debug build's change of mind,
    /// `docs/llm-seat.md` §"Changing a chair during the game").
    pub(crate) fn order(&mut self, line: &str) -> Result<(), String> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| "the bridge was let go".to_string())?;
        stdin
            .write_all(line.as_bytes())
            .and_then(|()| stdin.flush())
            .map_err(|e| format!("the bridge did not take the order: {e}"))
    }

    /// Whether the process has ended.
    pub(crate) fn exited(&mut self) -> bool {
        self.child
            .as_mut()
            .is_none_or(|child| !matches!(child.try_wait(), Ok(None)))
    }

    /// How many lines it has printed.
    pub(crate) fn heard(&self) -> usize {
        self.heard.load(Ordering::Acquire)
    }

    /// The last line it printed.
    pub(crate) fn last_line(&self) -> Option<String> {
        self.last
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Lets go: the stdin closes, and a tethered bridge gives its chair up
/// before the game or stops in it. One that has not gone five seconds
/// later is killed, by a thread of its own and never in a frame, which also
/// reaps it.
impl Drop for Bridge {
    fn drop(&mut self) {
        self.stdin = None;
        let Some(mut child) = self.child.take() else {
            return;
        };
        std::thread::spawn(move || {
            for _ in 0..50 {
                if !matches!(child.try_wait(), Ok(None)) {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            let _ = child.kill();
            let _ = child.wait();
        });
    }
}
