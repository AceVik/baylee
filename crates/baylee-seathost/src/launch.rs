//! How a seat agent runs a bridge and a check: [`ProcessLauncher`] starts
//! `baylee-seat` as a process, the one deployed; a test may hand the seat
//! agent another [`Launcher`] that plays in-process.

use crate::state::Probe;
use baylee_client_core::llmseat::blank_key_shapes;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::sync::oneshot;

/// A future a launcher answers with.
pub type Running<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// One hosted chair to play.
pub struct SeatJob {
    /// The gateway's order.
    pub order: String,
    /// The room.
    pub game_id: String,
    /// The chair.
    pub seat: u32,
    /// The profile's id.
    pub profile: String,
    /// The chair's name after `LLM-` ([`baylee_protocol::seathost::chair_name`]).
    pub name: String,
    /// The chair ticket, for the bridge's stdin only.
    pub chair_ticket: String,
    /// Where the bridge dials (`http://…`).
    pub gateway_url: String,
    /// A deck for `--deck`, or none for the bridge's own.
    pub deck_text: Option<String>,
    /// A settings file holding this profile alone, its caps as the file's.
    pub settings: String,
    /// The profile's own spend book.
    pub ledger: PathBuf,
}

impl std::fmt::Debug for SeatJob {
    /// Never the ticket.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SeatJob({}, {}#{}, {})",
            self.order, self.game_id, self.seat, self.profile
        )
    }
}

/// How a bridge ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Exit {
    /// Its game is over, or it was stopped.
    Ended,
    /// It failed, saying this last, read as a check's answer would be.
    Failed(Probe),
}

/// Runs bridges and checks.
pub trait Launcher: Send + Sync + 'static {
    /// Plays `job` until it ends or `stop` fires; says `started` once the
    /// bridge sits in its chair.
    fn run(
        &self,
        job: SeatJob,
        started: oneshot::Sender<()>,
        stop: oneshot::Receiver<()>,
    ) -> Running<Exit>;

    /// Checks a profile (the settings file `settings` holds it alone), with
    /// a paid canary when asked.
    fn probe(&self, settings: String, profile: String, canary: bool) -> Running<Probe>;
}

/// What a sentence a bridge said last is, as a check would have said it:
/// the bridge's own `cause` words (`baylee-seat check`).
#[must_use]
pub fn read_cause(cause: &str, why: String) -> Probe {
    match cause {
        "signed_out" => Probe::SignedOut(why),
        "limited" => Probe::Limited {
            why,
            lifts_secs: None,
        },
        _ => Probe::Failing(why),
    }
}

/// The cause a free sentence names, as `baylee-seat check` reads one.
#[must_use]
pub fn cause_in(why: &str) -> &'static str {
    let lower = why.to_ascii_lowercase();
    if [
        "signed out",
        "not signed in",
        "sign in",
        "not logged in",
        "log in",
        "login",
    ]
    .iter()
    .any(|w| lower.contains(w))
    {
        "signed_out"
    } else if ["rate limit", "quota", "overloaded", "credit", "usage limit"]
        .iter()
        .any(|w| lower.contains(w))
    {
        "limited"
    } else {
        "failing"
    }
}

/// `baylee-seat` as a process.
pub struct ProcessLauncher {
    /// The bridge's binary.
    pub bridge: PathBuf,
    /// Where each run's settings file and deck are written (`0700`).
    pub work: PathBuf,
    /// What of the seat agent's environment a bridge is given: its home
    /// (a CLI's sign-in lives there), `PATH`, the key store's switch, the
    /// locale and the log filter. Nothing else, and never a key.
    pub env: Vec<(String, String)>,
}

/// The variables a bridge inherits from its seat agent.
pub const PASSED_ON: &[&str] = &[
    "HOME",
    "PATH",
    "USER",
    "LOGNAME",
    "LANG",
    "LC_ALL",
    "TZ",
    "RUST_LOG",
    "BAYLEE_KEY_STORE",
    "CLAUDE_CONFIG_DIR",
    "CODEX_HOME",
    "JUNIE_HOME",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_STATE_HOME",
    "XDG_CACHE_HOME",
    "SystemRoot",
];

/// How long a stopped bridge has to give its chair back before it is
/// killed.
const STOP_GRACE: Duration = Duration::from_secs(30);

/// How long a check may take, the canary's call included.
const PROBE_WAIT: Duration = Duration::from_secs(90);

impl ProcessLauncher {
    /// Reads [`PASSED_ON`] from this process's environment.
    #[must_use]
    pub fn new(bridge: PathBuf, work: PathBuf) -> Self {
        let env = PASSED_ON
            .iter()
            .filter_map(|name| std::env::var(name).ok().map(|v| ((*name).to_string(), v)))
            .collect();
        Self { bridge, work, env }
    }

    /// A directory of its own under [`Self::work`], `0700`.
    fn scratch(&self, name: &str) -> std::io::Result<PathBuf> {
        let dir = self.work.join(name);
        let _ = std::fs::remove_dir_all(&dir);
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        builder.create(&dir)?;
        Ok(dir)
    }

    fn command(&self) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(&self.bridge);
        command
            .env_clear()
            .envs(self.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .kill_on_drop(true);
        command
    }
}

/// Writes `text` to `path`, `0600`.
fn write_private(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    std::io::Write::write_all(&mut options.open(path)?, text.as_bytes())
}

impl Launcher for ProcessLauncher {
    fn run(
        &self,
        job: SeatJob,
        started: oneshot::Sender<()>,
        stop: oneshot::Receiver<()>,
    ) -> Running<Exit> {
        let prepared = self
            .scratch(&format!("seat-{}", job.order))
            .and_then(|dir| {
                let settings = dir.join("llm-seat.json");
                write_private(&settings, &job.settings)?;
                let deck = match &job.deck_text {
                    Some(text) => {
                        let path = dir.join("deck.txt");
                        write_private(&path, text)?;
                        Some(path)
                    }
                    None => None,
                };
                Ok((dir, settings, deck))
            });
        let mut command = self.command();
        Box::pin(async move {
            let (dir, settings, deck) = match prepared {
                Ok(prepared) => prepared,
                Err(e) => {
                    return Exit::Failed(Probe::Failing(format!(
                        "the seat agent could not write the bridge's files: {}",
                        e.kind()
                    )));
                }
            };
            command
                .arg("join")
                .arg(&job.game_id)
                .args(["--chair", &job.seat.to_string()])
                .args(["--chair-ticket", "--tethered", "--hosted"])
                .args(["--gateway", &job.gateway_url])
                .arg("--config")
                .arg(&settings)
                .args(["--profile", &job.profile])
                .arg("--ledger")
                .arg(&job.ledger)
                .args(["--name", &job.name]);
            if let Some(deck) = &deck {
                command.arg("--deck").arg(deck);
            }
            let exit = play(command, &job, started, stop).await;
            let _ = std::fs::remove_dir_all(&dir);
            exit
        })
    }

    fn probe(&self, settings: String, profile: String, canary: bool) -> Running<Probe> {
        let prepared = self
            .scratch(&format!("probe-{}", uuid::Uuid::now_v7()))
            .and_then(|dir| {
                let path = dir.join("llm-seat.json");
                write_private(&path, &settings)?;
                Ok((dir, path))
            });
        let mut command = self.command();
        Box::pin(async move {
            let (dir, path) = match prepared {
                Ok(prepared) => prepared,
                Err(e) => {
                    return Probe::Failing(format!(
                        "the seat agent could not write the check's files: {}",
                        e.kind()
                    ));
                }
            };
            command
                .args(["check", "--profile", &profile, "--config"])
                .arg(&path)
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .stdout(Stdio::piped());
            if canary {
                command.arg("--canary");
            }
            let said = match tokio::time::timeout(PROBE_WAIT, command.output()).await {
                Ok(Ok(out)) => String::from_utf8_lossy(&out.stdout).into_owned(),
                Ok(Err(e)) => format!(
                    "{{\"ok\":false,\"error\":\"the bridge did not start: {}\"}}",
                    e.kind()
                ),
                Err(_) => "{\"ok\":false,\"error\":\"the check did not answer in time\"}".into(),
            };
            let _ = std::fs::remove_dir_all(&dir);
            read_check(&said)
        })
    }
}

/// What `baylee-seat check` printed, read.
#[must_use]
pub fn read_check(said: &str) -> Probe {
    let line = said.lines().rev().find(|l| l.trim_start().starts_with('{'));
    let Some(value) = line.and_then(|l| serde_json::from_str::<serde_json::Value>(l).ok()) else {
        return Probe::Failing("the check said nothing readable".into());
    };
    if value["ok"] == true {
        return Probe::Ok;
    }
    let why = blank_key_shapes(value["error"].as_str().unwrap_or("the check failed"));
    let cause = value["cause"]
        .as_str()
        .map_or_else(|| cause_in(&why), |c| c);
    read_cause(cause, why)
}

/// Runs the bridge `command` for `job`: the ticket on stdin's first line,
/// stdin held while it plays (a tethered bridge stops when it closes).
async fn play(
    mut command: tokio::process::Command,
    job: &SeatJob,
    started: oneshot::Sender<()>,
    mut stop: oneshot::Receiver<()>,
) -> Exit {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            return Exit::Failed(Probe::Failing(format!(
                "the bridge did not start: {}",
                e.kind()
            )));
        }
    };
    let mut stdin = child.stdin.take();
    if let Some(stdin) = stdin.as_mut() {
        let line = format!("{}\n", job.chair_ticket);
        if stdin.write_all(line.as_bytes()).await.is_err() {
            let _ = child.kill().await;
            return Exit::Failed(Probe::Failing("the bridge closed its stdin".into()));
        }
    }
    // Its lines: the last one said is the detail; "sits in chair" is the
    // moment it took the chair.
    let last = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let mut started = Some(started);
    let (sat_tx, mut sat_rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    for stream in [
        child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Send + Unpin>),
        child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Send + Unpin>),
    ]
    .into_iter()
    .flatten()
    {
        let last = last.clone();
        let sat = sat_tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.contains("sits in chair") {
                    let _ = sat.send(());
                }
                if !line.trim().is_empty() {
                    *last
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) =
                        blank_key_shapes(&line).chars().take(300).collect();
                }
            }
        });
    }
    drop(sat_tx);
    let status = loop {
        tokio::select! {
            status = child.wait() => break status,
            Some(()) = sat_rx.recv() => {
                if let Some(started) = started.take() {
                    let _ = started.send(());
                }
            }
            _ = &mut stop, if stdin.is_some() => {
                // Let go: a tethered bridge gives its chair back or ends.
                drop(stdin.take());
                if let Ok(status) = tokio::time::timeout(STOP_GRACE, child.wait()).await {
                    break status;
                }
                let _ = child.kill().await;
                break child.wait().await;
            }
        }
    };
    drop(stdin);
    let said = last
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    match status {
        Ok(status) if status.success() => Exit::Ended,
        Ok(_) => {
            let said = said
                .strip_prefix("Error: ")
                .map_or(said.clone(), str::to_string);
            let said = if said.is_empty() {
                "the bridge ended without saying why".to_string()
            } else {
                said
            };
            Exit::Failed(read_cause(cause_in(&said), said))
        }
        Err(e) => Exit::Failed(Probe::Failing(format!(
            "the bridge could not be waited for: {}",
            e.kind()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_checks_line_is_read_and_a_key_in_it_blanked() {
        assert_eq!(read_check("hello\n{\"ok\":true}\n"), Probe::Ok);
        assert_eq!(
            read_check(r#"{"ok":false,"cause":"signed_out","error":"claude is not signed in"}"#),
            Probe::SignedOut("claude is not signed in".into())
        );
        let Probe::Failing(why) = read_check(
            r#"{"ok":false,"error":"refused key sk-ant-api03-abcdefghijklmnopqrstuvwxyz0123"}"#,
        ) else {
            panic!("failing");
        };
        assert!(!why.contains("abcdefghijklmnop"), "{why}");
        assert!(matches!(
            read_check(r#"{"ok":false,"cause":"limited","error":"rate limit"}"#),
            Probe::Limited { .. }
        ));
        assert!(matches!(read_check(""), Probe::Failing(_)));
    }
}
