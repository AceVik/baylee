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
    /// The profile's id, as ordered.
    pub profile: String,
    /// The profile that plays the game: [`Self::profile`], or its fallback
    /// from the start (`crate::Host::place`). The settings file and the
    /// spend book are its.
    pub plays: String,
    /// The chair's name after `LLM-` ([`baylee_protocol::seathost::chair_name`]).
    pub name: String,
    /// The chair ticket, for the bridge's stdin only.
    pub chair_ticket: String,
    /// Where the bridge dials (`http://…`).
    pub gateway_url: String,
    /// A deck for `--deck`, or none for the bridge's own.
    pub deck_text: Option<String>,
    /// A settings file holding the profile that plays alone, its caps as
    /// the file's.
    pub settings: String,
    /// That profile's own spend book.
    pub ledger: PathBuf,
    /// The fallbacks that stand by in the game, in the order they take
    /// over.
    pub fallbacks: Vec<FallbackJob>,
    /// Where what the bridge says of its model goes ([`Heard`]), where
    /// anybody listens.
    pub heard: Option<tokio::sync::mpsc::UnboundedSender<Heard>>,
}

/// A fallback profile a game takes along: a `--fallback-profile`, with its
/// own settings file and spend book, so its caps are its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FallbackJob {
    /// Its id.
    pub profile: String,
    /// A settings file holding it alone, its caps as the file's.
    pub settings: String,
    /// Its own spend book.
    pub ledger: PathBuf,
}

/// What a bridge says of its model while it plays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Heard {
    /// The model playing is rate limited, and a fallback was asked to take
    /// over: until the provider's time, where it named one.
    Limited {
        /// Whose model: `None` for the profile the game plays, else the
        /// fallback that had taken over from it.
        of: Option<String>,
        /// Seconds until it lifts, where said.
        lifts_secs: Option<i64>,
    },
}

/// The words a bridge says when its model is rate limited and a fallback
/// stood by (`baylee-seat`'s `bridge::LIMITED`), then `(it lifts in N s)`
/// where the provider said. A model that failed otherwise is not limited,
/// and its bridge never says them.
pub const LIMITED: &str = "the primary is rate limited";

/// What a bridge says when a fallback that took over is rate limited in
/// its turn and another stood by behind it (`baylee-seat`'s
/// `bridge::fallback_limited`): `the fallback <profile>` before these words.
pub const FALLBACK_LIMITED: &str = " is rate limited";

/// What `line` says of the model playing, if it is the bridge's
/// [`LIMITED`], or the same of a fallback that took over
/// ([`FALLBACK_LIMITED`]).
#[must_use]
pub fn heard_in(line: &str) -> Option<Heard> {
    let (of, after) = if let Some((_, after)) = line.split_once(LIMITED) {
        (None, after)
    } else {
        let (before, after) = line.split_once(FALLBACK_LIMITED)?;
        let (_, named) = before.rsplit_once("the fallback ")?;
        let named = named.trim();
        if !baylee_protocol::seathost::valid_name(named) {
            return None;
        }
        (Some(named.to_owned()), after)
    };
    let lifts_secs = after
        .split_once("(it lifts in ")
        .and_then(|(_, rest)| rest.split_once(" s)"))
        .and_then(|(secs, _)| secs.trim().parse::<i64>().ok());
    Some(Heard::Limited { of, lifts_secs })
}

impl std::fmt::Debug for SeatJob {
    /// Never the ticket.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SeatJob({}, {}#{}, {}",
            self.order, self.game_id, self.seat, self.profile
        )?;
        if self.plays != self.profile {
            write!(f, " plays {}", self.plays)?;
        }
        for (at, fallback) in self.fallbacks.iter().enumerate() {
            let then = if at == 0 { ", fallback" } else { ", then" };
            write!(f, "{then} {}", fallback.profile)?;
        }
        f.write_str(")")
    }
}

/// How a bridge ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Exit {
    /// Its game is over, or it was stopped, the bridge saying this last:
    /// its closing lines ([`CLOSING_LINES`], oldest first, joined by ` | `),
    /// among them its summary of the game (how many answers the mind gave
    /// and how many the house), blanked of anything shaped like a key, as a
    /// failure's.
    Ended(String),
    /// It failed, saying this last, read as a check's answer would be.
    Failed(Probe),
}

/// How many of a bridge's last lines a game that ended well leaves in the
/// operator's log: its summary and what follows it (why the house
/// answered, the tokens, a rule of thumb, the spend book settled), with
/// room for more.
pub const CLOSING_LINES: usize = 8;

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

/// How long an ended bridge's last lines are waited for: a child of its
/// own holding the pipe open does not hold the seat agent.
const READ_GRACE: Duration = Duration::from_secs(2);

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

    /// `baylee-seat join` for `job`, its files at `files`: the settings
    /// file of the profile that plays, its book, the chair's name and deck,
    /// and each fallback's file and book, in order, where any stands by
    /// (each `--fallback-config` and `--fallback-ledger` with the
    /// `--fallback-profile` at its place). Nothing else:
    /// the environment is [`Self::env`] alone, and no key is in either.
    #[must_use]
    pub fn join_command(&self, job: &SeatJob, files: &JobFiles) -> tokio::process::Command {
        let mut command = self.command();
        command
            .arg("join")
            .arg(&job.game_id)
            .args(["--chair", &job.seat.to_string()])
            .args(["--chair-ticket", "--tethered"])
            .args(["--gateway", &job.gateway_url])
            .arg("--config")
            .arg(&files.settings)
            .args(["--profile", &job.plays])
            .arg("--ledger")
            .arg(&job.ledger)
            .args(["--name", &job.name]);
        if let Some(deck) = &files.deck {
            command.arg("--deck").arg(deck);
        }
        for (fallback, settings) in job.fallbacks.iter().zip(&files.fallbacks) {
            command
                .args(["--fallback-profile", &fallback.profile])
                .arg("--fallback-config")
                .arg(settings)
                .arg("--fallback-ledger")
                .arg(&fallback.ledger);
        }
        command
    }

    /// Writes `job`'s files into `dir` (`0600` each).
    fn write_job(dir: &std::path::Path, job: &SeatJob) -> std::io::Result<JobFiles> {
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
        // Named by place, never by id: an id is not a file's name.
        let mut fallbacks = Vec::with_capacity(job.fallbacks.len());
        for (at, fallback) in job.fallbacks.iter().enumerate() {
            let path = dir.join(format!("llm-seat-fallback-{}.json", at + 1));
            write_private(&path, &fallback.settings)?;
            fallbacks.push(path);
        }
        Ok(JobFiles {
            settings,
            deck,
            fallbacks,
        })
    }
}

/// Where a job's files were written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobFiles {
    /// The settings file of the profile that plays.
    pub settings: PathBuf,
    /// The deck, where the order brought one.
    pub deck: Option<PathBuf>,
    /// Each fallback's settings file, in the order they take over.
    pub fallbacks: Vec<PathBuf>,
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
            .and_then(|dir| Ok((Self::write_job(&dir, &job)?, dir)))
            .map(|(files, dir)| (self.join_command(&job, &files), dir));
        Box::pin(async move {
            let (command, dir) = match prepared {
                Ok(prepared) => prepared,
                Err(e) => {
                    return Exit::Failed(Probe::Failing(format!(
                        "the seat agent could not write the bridge's files: {}",
                        e.kind()
                    )));
                }
            };
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
    let mut started = Some(started);
    let (sat_tx, mut sat_rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let (last, readers) = read_lines(&mut child, &sat_tx, job.heard.as_ref());
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
    // The pipes close with the bridge; what it said last is read before
    // it is told.
    for reader in readers {
        let _ = tokio::time::timeout(READ_GRACE, reader).await;
    }
    let closing = last
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let said = closing.back().cloned().unwrap_or_default();
    match status {
        Ok(status) if status.success() => {
            Exit::Ended(closing.into_iter().collect::<Vec<_>>().join(" | "))
        }
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

/// A bridge's closing lines, oldest first, shared with their readers.
type Closing = std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<String>>>;

/// Reads what `child` says on both its streams: its last
/// [`CLOSING_LINES`], blanked of key shapes, and a word on `sat` each time it
/// says it sits in its chair. The summary of a game that ended well comes a
/// few lines before the last (the spend book is settled after it).
fn read_lines(
    child: &mut tokio::process::Child,
    sat: &tokio::sync::mpsc::UnboundedSender<()>,
    heard: Option<&tokio::sync::mpsc::UnboundedSender<Heard>>,
) -> (Closing, Vec<tokio::task::JoinHandle<()>>) {
    let last: Closing = std::sync::Arc::new(std::sync::Mutex::new(
        std::collections::VecDeque::with_capacity(CLOSING_LINES),
    ));
    let mut readers = Vec::new();
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
        let sat = sat.clone();
        let heard = heard.cloned();
        readers.push(tokio::spawn(async move {
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.contains("sits in chair") {
                    let _ = sat.send(());
                }
                if let (Some(heard), Some(said)) = (&heard, heard_in(&line)) {
                    let _ = heard.send(said);
                }
                if !line.trim().is_empty() {
                    let mut last = last
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if last.len() == CLOSING_LINES {
                        last.pop_front();
                    }
                    last.push_back(blank_key_shapes(&line).chars().take(300).collect());
                }
            }
        }));
    }
    (last, readers)
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

    /// A bridge that ends well leaves its summary for the operator's log:
    /// who answered how often, a key in it blanked as in a failure.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_bridge_that_ends_well_leaves_its_last_line() {
        let job = SeatJob {
            order: "order".into(),
            game_id: "game".into(),
            seat: 1,
            profile: "deepseek".into(),
            plays: "deepseek".into(),
            name: "DeepSeek".into(),
            chair_ticket: "ticket".into(),
            gateway_url: "http://127.0.0.1:1".into(),
            deck_text: None,
            settings: String::new(),
            ledger: PathBuf::from("ledger.json"),
            fallbacks: Vec::new(),
            heard: None,
        };
        let mut command = tokio::process::Command::new("sh");
        command.args([
            "-c",
            "read ticket; echo \"sits in chair\"; sleep 0.3; \
             echo \"won after 3 turns: the mind answered 7, the house 1; \
             key sk-ant-api03-abcdefghijklmnopqrstuvwxyz0123\"; \
             echo \"house answers: 7x unavailable: 504\"; \
             echo \"tokens: 1308359 in, 4887 out; 42 of 1000 calls (7 failed)\"; \
             echo \"rule of thumb: over 10 % house answers\"; \
             echo \"the game is settled in the spend book\"",
        ]);
        let (started, started_rx) = oneshot::channel();
        let (_stop, stop) = oneshot::channel();
        let Exit::Ended(said) = play(command, &job, started, stop).await else {
            panic!("the bridge ended well");
        };
        assert!(started_rx.await.is_ok(), "it sat in its chair");
        assert!(said.contains("the mind answered 7, the house 1"), "{said}");
        assert!(
            said.ends_with("the game is settled in the spend book"),
            "{said}"
        );
        assert!(!said.contains("abcdefghijklmnop"), "{said}");
    }

    /// A job as the seat agent builds it for a chair ordered as
    /// `primary`: with `fallback`, the game plays the profile and
    /// `backup` stands by; without, `backup` plays it outright.
    fn job(fallback: bool) -> SeatJob {
        SeatJob {
            order: "order-7".into(),
            game_id: "game-9".into(),
            seat: 2,
            profile: "primary".into(),
            plays: if fallback { "primary" } else { "backup" }.into(),
            name: "Model-A".into(),
            chair_ticket: "the-secret-chair-ticket".into(),
            gateway_url: "http://127.0.0.1:28766".into(),
            deck_text: None,
            settings: r#"{"default":"settings-of-the-player"}"#.into(),
            ledger: PathBuf::from("/state/spend/own.json"),
            fallbacks: fallback
                .then(|| FallbackJob {
                    profile: "backup".into(),
                    settings: r#"{"default":"settings-of-the-fallback"}"#.into(),
                    ledger: PathBuf::from("/state/spend/backup.json"),
                })
                .into_iter()
                .collect(),
            heard: None,
        }
    }

    fn launcher() -> ProcessLauncher {
        ProcessLauncher {
            bridge: PathBuf::from("/opt/baylee/baylee-seat"),
            work: PathBuf::from("/state/run"),
            env: vec![
                ("HOME".into(), "/home/seat".into()),
                ("BAYLEE_KEY_STORE".into(), "file:/x".into()),
            ],
        }
    }

    fn args_of(command: &tokio::process::Command) -> Vec<String> {
        command
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    /// The bridge is started with the profile that plays and its own files,
    /// the chair's name, and, where one stands by, the fallback's profile,
    /// settings file and spend book: nothing about the ticket, a key or the
    /// settings' text in its arguments, and only the allowlisted variables
    /// set. (Before, a fallback reached the bridge in no way.)
    #[test]
    fn the_bridge_is_started_with_the_profile_that_plays_and_its_fallback() {
        let files = JobFiles {
            settings: PathBuf::from("/run/seat-order-7/llm-seat.json"),
            deck: None,
            fallbacks: vec![PathBuf::from("/run/seat-order-7/llm-seat-fallback-1.json")],
        };
        let command = launcher().join_command(&job(true), &files);
        assert_eq!(
            command.as_std().get_program(),
            std::ffi::OsStr::new("/opt/baylee/baylee-seat")
        );
        assert_eq!(
            args_of(&command),
            [
                "join",
                "game-9",
                "--chair",
                "2",
                "--chair-ticket",
                "--tethered",
                "--gateway",
                "http://127.0.0.1:28766",
                "--config",
                "/run/seat-order-7/llm-seat.json",
                "--profile",
                "primary",
                "--ledger",
                "/state/spend/own.json",
                "--name",
                "Model-A",
                "--fallback-profile",
                "backup",
                "--fallback-config",
                "/run/seat-order-7/llm-seat-fallback-1.json",
                "--fallback-ledger",
                "/state/spend/backup.json",
            ]
        );
        let joined = args_of(&command).join(" ");
        assert!(
            !joined.contains("secret-chair-ticket"),
            "the ticket is on stdin"
        );
        assert!(
            !joined.contains("settings-of"),
            "a settings file's text is no argument"
        );

        let mut set: Vec<(String, Option<String>)> = command
            .as_std()
            .get_envs()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().into_owned(),
                    v.map(|v| v.to_string_lossy().into_owned()),
                )
            })
            .collect();
        set.sort();
        assert_eq!(
            set,
            [
                ("BAYLEE_KEY_STORE".to_string(), Some("file:/x".to_string())),
                ("HOME".to_string(), Some("/home/seat".to_string())),
            ],
            "the allowlist and nothing else"
        );
    }

    /// A game that plays the profile ordered, when the fallback is not
    /// standing by, and one with a deck, are started with exactly those
    /// arguments: no fallback flag where there is no fallback, the deck's file
    /// where there is a deck. The profile that plays is the one named, not
    /// the one ordered.
    #[test]
    fn a_bridge_without_a_fallback_is_given_no_fallback_flags_and_the_profile_that_plays() {
        let files = JobFiles {
            settings: PathBuf::from("/run/s/llm-seat.json"),
            deck: Some(PathBuf::from("/run/s/deck.txt")),
            fallbacks: Vec::new(),
        };
        // Ordered as the primary, played by the fallback: `plays` is named.
        let command = launcher().join_command(&job(false), &files);
        let args = args_of(&command);
        let at = |flag: &str| {
            let place = args.iter().position(|a| a == flag).expect(flag);
            args[place + 1].as_str()
        };
        assert_eq!(at("--profile"), "backup");
        assert_eq!(at("--ledger"), "/state/spend/own.json");
        assert_eq!(at("--name"), "Model-A");
        assert_eq!(at("--deck"), "/run/s/deck.txt");
        assert!(
            args.iter().all(|a| !a.starts_with("--fallback")),
            "{args:?}"
        );
    }

    /// The files a job needs are written `0600`, the fallback's settings in a
    /// file of their own beside the player's, and only when one stands by.
    #[cfg(unix)]
    #[test]
    fn a_jobs_files_are_written_privately_the_fallbacks_beside_the_players() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = std::env::temp_dir().join(format!("baylee-launch-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut with_deck = job(true);
        with_deck.deck_text = Some("60 Forest".into());

        let files = ProcessLauncher::write_job(&dir, &with_deck).unwrap();

        let (settings, fallback) = (files.settings.clone(), files.fallbacks[0].clone());
        assert_ne!(settings, fallback);
        assert_eq!(
            std::fs::read_to_string(&settings).unwrap(),
            r#"{"default":"settings-of-the-player"}"#
        );
        assert_eq!(
            std::fs::read_to_string(&fallback).unwrap(),
            r#"{"default":"settings-of-the-fallback"}"#
        );
        assert_eq!(
            std::fs::read_to_string(files.deck.unwrap()).unwrap(),
            "60 Forest"
        );
        for path in [&settings, &fallback] {
            let mode = std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "{}", path.display());
        }
        let alone = ProcessLauncher::write_job(&dir, &job(false)).unwrap();
        assert!(alone.fallbacks.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Nothing of the seat agent's environment reaches a bridge but what the
    /// launcher names: run, the bridge's environment holds `HOME` and the key
    /// store's switch and not what the test process itself was started with.
    /// (`get_envs` lists only what was set, and cannot tell a cleared
    /// environment from an inherited one: this reads what a process sees.)
    #[cfg(unix)]
    #[tokio::test]
    async fn a_bridge_sees_the_allowlisted_environment_and_nothing_else() {
        // Cargo sets this in every test process it runs.
        assert!(std::env::var_os("CARGO_MANIFEST_DIR").is_some());
        let mut launch = launcher();
        launch.bridge = PathBuf::from("/bin/sh");
        let mut command = launch.command();
        let out = command
            .args(["-c", "export -p"])
            .output()
            .await
            .expect("sh runs");
        let seen = String::from_utf8_lossy(&out.stdout).into_owned();
        assert!(
            seen.contains("HOME") && seen.contains("/home/seat"),
            "{seen}"
        );
        assert!(seen.contains("BAYLEE_KEY_STORE"), "{seen}");
        assert!(!seen.contains("CARGO_MANIFEST_DIR"), "inherited: {seen}");
        assert!(
            !seen.contains("ANTHROPIC") && !seen.contains("_API_KEY"),
            "{seen}"
        );
    }

    /// The bridge's own words that its model is rate limited and its fallback
    /// asked to take over are read, with the seconds when the provider named
    /// them, from the line wherever it stands in it; no other line is.
    #[test]
    fn a_rate_limited_primary_is_heard_with_the_seconds_it_names() {
        assert_eq!(
            heard_in("the primary is rate limited (it lifts in 120 s)"),
            Some(Heard::Limited {
                of: None,
                lifts_secs: Some(120)
            })
        );
        assert_eq!(
            heard_in("seat: the primary is rate limited; the fallback backup plays on"),
            Some(Heard::Limited {
                of: None,
                lifts_secs: None
            }),
            "no parenthesis, no seconds"
        );
        assert_eq!(
            heard_in("12:01:07 note: the primary is rate limited (it lifts in 45 s) so"),
            Some(Heard::Limited {
                of: None,
                lifts_secs: Some(45)
            }),
            "a prefix does not matter"
        );
        assert_eq!(
            heard_in("the primary is rate limited (it lifts in soon s)"),
            Some(Heard::Limited {
                of: None,
                lifts_secs: None
            }),
            "seconds that are no number are none"
        );
        for line in [
            "sits in chair",
            "",
            "the model is rate limited",
            "won after 3 turns: the mind answered 7, the house 1",
        ] {
            assert_eq!(heard_in(line), None, "«{line}»");
        }
    }

    /// A fallback that took over and is limited in its turn is heard by its
    /// own profile, wherever the line names it: the profile held back is the
    /// one that is limited. A provider's words that only end the same are
    /// no profile's.
    #[test]
    fn a_rate_limited_fallback_is_heard_by_its_own_name() {
        let fb1 = |lifts_secs| {
            Some(Heard::Limited {
                of: Some("fb1".into()),
                lifts_secs,
            })
        };
        assert_eq!(
            heard_in(
                "seat: the fallback fb2 answers from turn 7: the fallback fb1 is rate limited \
                 (it lifts in 60 s)"
            ),
            fb1(Some(60))
        );
        assert_eq!(
            heard_in(
                "the fallback fb2 could not take over in turn 9: the day's cap is spent; the \
                 fallback fb1 is rate limited"
            ),
            fb1(None)
        );
        for line in [
            "the fallback fb2 answers from turn 3: the primary failed: 429: the org is rate \
             limited",
            "the fallback  is rate limited",
        ] {
            assert_eq!(heard_in(line), None, "«{line}»");
        }
    }

    /// A chain of fallbacks reaches the bridge in its order, each with its
    /// own settings file and spend book beside its `--fallback-profile`; the
    /// files are named by place, never by a profile's id.
    #[test]
    fn a_chain_of_fallbacks_is_handed_to_the_bridge_in_its_order() {
        let mut chain = job(true);
        chain.fallbacks.push(FallbackJob {
            profile: "spare".into(),
            settings: r#"{"default":"settings-of-the-spare"}"#.into(),
            ledger: PathBuf::from("/state/spend/spare.json"),
        });
        let files = JobFiles {
            settings: PathBuf::from("/run/s/llm-seat.json"),
            deck: None,
            fallbacks: vec![
                PathBuf::from("/run/s/llm-seat-fallback-1.json"),
                PathBuf::from("/run/s/llm-seat-fallback-2.json"),
            ],
        };
        let args = args_of(&launcher().join_command(&chain, &files));
        let from = args
            .iter()
            .position(|a| a == "--fallback-profile")
            .expect("a fallback");
        assert_eq!(
            args[from..],
            [
                "--fallback-profile",
                "backup",
                "--fallback-config",
                "/run/s/llm-seat-fallback-1.json",
                "--fallback-ledger",
                "/state/spend/backup.json",
                "--fallback-profile",
                "spare",
                "--fallback-config",
                "/run/s/llm-seat-fallback-2.json",
                "--fallback-ledger",
                "/state/spend/spare.json",
            ]
        );
        assert!(format!("{chain:?}").contains("fallback backup, then spare"));
    }

    /// What a running bridge says reaches the channel of its job: its rate
    /// limited line, said on either of its streams, is sent as `Heard`, while
    /// the seat agent's other business (sitting, closing lines) is unharmed.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_bridge_that_says_it_is_limited_is_heard_by_its_job() {
        let (heard, mut listening) = tokio::sync::mpsc::unbounded_channel();
        let mut job = job(true);
        job.heard = Some(heard);
        let mut command = tokio::process::Command::new("sh");
        command.args([
            "-c",
            "read ticket; echo \"sits in chair\"; \
             echo \"the primary is rate limited (it lifts in 120 s)\" >&2; \
             echo \"the game is settled in the spend book\"",
        ]);
        let (started, _started_rx) = oneshot::channel();
        let (_stop, stop) = oneshot::channel();
        let Exit::Ended(said) = play(command, &job, started, stop).await else {
            panic!("the bridge ended well");
        };
        assert_eq!(
            listening.try_recv(),
            Ok(Heard::Limited {
                of: None,
                lifts_secs: Some(120)
            })
        );
        assert!(listening.try_recv().is_err(), "said once, heard once");
        assert!(said.contains("the primary is rate limited"), "{said}");
    }

    /// A job's debug line, which the seat agent logs at every start, names
    /// the profile that plays when it is not the one ordered and the fallback
    /// that stands by, and never the ticket.
    #[test]
    fn a_jobs_debug_line_names_where_it_plays_and_never_the_ticket() {
        let mut diverted = job(false);
        let line = format!("{diverted:?}");
        assert!(line.contains("primary plays backup"), "{line}");
        assert!(!line.contains("fallback"), "{line}");
        diverted = job(true);
        let line = format!("{diverted:?}");
        assert!(line.contains("fallback backup"), "{line}");
        assert!(!line.contains("plays"), "{line}");
        assert!(!line.contains("secret-chair-ticket"), "{line}");
    }
}
