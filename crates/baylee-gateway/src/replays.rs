//! Replay jobs (`docs/protocol.md` §"Replays"): a hosted game the asking
//! account sat at, played again on this build by an engine an agent starts
//! for the job, which sends back the tape the asked perspective is sent.
//!
//! ```text
//! client ──POST /replays/{g}/tape──> gateway ──StartEngine{"replay-…"}──> agent ── spawn ──> engine
//!                                      <── EngineHello{"replay-…", token, build} ──────────────┘
//!                                      ── ProjectRecord{job, record, perspective, seat, …} ──>
//!                                      <── TapeChunk{job, seq, data, last} … ──
//! client <──200 the tape──
//! ```
//!
//! The record is read here, after the route found the asking account's seat
//! in the table (`records::seat_of`), and goes only to the job's engine, on
//! the engine link it came from; a player is sent the tape, never the record
//! (`records::for_projection` has its one caller here, which a test holds).
//! Nothing is stored: the tape is gathered in memory and answered.
//!
//! To an agent a job is a game: it starts an engine for its id, counts it
//! against its capacity and reports it as it reports any engine. Jobs are not
//! games to anybody else: no lobby entry, no `/health` count, no record.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use baylee_protocol::v1::{self, Envelope};
use parking_lot::Mutex;
use tokio::sync::oneshot;

use crate::{Shared, auth};

/// The largest record a job takes, compressed: past it `413`.
pub const MAX_RECORD_BYTES: u64 = 8 * 1024 * 1024;

/// The largest tape a job may answer, compressed: past it `502`.
pub const MAX_TAPE_BYTES: usize = 32 * 1024 * 1024;

/// How long a job has from its order to its tape's last piece when
/// `BAYLEE_REPLAY_JOB_SECS` says nothing: past it `504`, and its engine is
/// told to stop.
pub const JOB_TIMEOUT: Duration = Duration::from_secs(60);

/// How many jobs one account may ask for per [`ASK_WINDOW`].
pub const ASKS_PER_WINDOW: usize = 6;

/// The window [`ASKS_PER_WINDOW`] counts in.
pub const ASK_WINDOW: Duration = Duration::from_secs(60);

/// How many of one account's jobs may run at once.
pub const JOBS_PER_ACCOUNT: u32 = 2;

/// How many jobs run at once on a gateway when `BAYLEE_REPLAY_JOBS` says
/// nothing.
pub const DEFAULT_JOBS: usize = 4;

/// What prefixes a job's id, so an engine's hello for one is told from a
/// game's at a glance in a log.
pub const JOB_PREFIX: &str = "replay-";

/// Why a job gave no tape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobError {
    /// No agent has room.
    NoCapacity,
    /// Admission is held for a deploy.
    Held,
    /// The engine sent no tape in time.
    Timeout,
    /// The tape passed [`MAX_TAPE_BYTES`].
    TooLarge,
    /// The engine failed, went away, or said something else.
    Failed,
}

/// One job waiting for its engine and its tape.
struct Job {
    /// SHA-256 of the token its engine proves itself with.
    token_hash: String,
    /// What the engine is sent once it has said hello; taken then.
    order: Option<v1::ProjectRecord>,
    /// Who waits for the tape; taken when it is answered.
    done: Option<oneshot::Sender<Result<Vec<u8>, JobError>>>,
}

/// The jobs running now, and the bounds on them.
pub struct Jobs {
    /// By job id.
    running: Mutex<HashMap<String, Job>>,
    /// Running jobs per account, for [`JOBS_PER_ACCOUNT`].
    per_account: Mutex<HashMap<String, u32>>,
    /// Asks per account, for [`ASKS_PER_WINDOW`].
    asks: auth::RateLimiter,
    /// `BAYLEE_REPLAY_JOBS`: how many run at once; 0 = replays are off.
    pub max: usize,
    /// `BAYLEE_REPLAY_JOB_SECS`: how long a job may take ([`JOB_TIMEOUT`]).
    pub timeout: Duration,
}

impl Jobs {
    /// The registry with room for `max` jobs at once.
    #[must_use]
    pub fn new(max: usize, timeout: Duration) -> Self {
        Self {
            running: Mutex::new(HashMap::new()),
            per_account: Mutex::new(HashMap::new()),
            asks: auth::RateLimiter::new(ASK_WINDOW, ASKS_PER_WINDOW),
            max,
            timeout,
        }
    }

    /// Gives `account`'s latest ask back: no engine took it.
    pub fn give_back(&self, account: &str) {
        self.asks.give_back(account);
    }

    /// Whether replays are served at all.
    #[must_use]
    pub const fn on(&self) -> bool {
        self.max > 0
    }
}

/// `BAYLEE_REPLAY_JOBS`: unset is [`DEFAULT_JOBS`], `0` is off.
///
/// # Errors
/// A value that is not a number from 0 to 64.
pub fn jobs_from_env(raw: Option<&str>) -> Result<usize, String> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(DEFAULT_JOBS),
        Some(text) => match text.parse::<usize>() {
            Ok(n) if n <= 64 => Ok(n),
            _ => Err(format!("{text:?} is not a number of jobs from 0 to 64")),
        },
    }
}

/// `BAYLEE_REPLAY_JOB_SECS`: unset is [`JOB_TIMEOUT`].
///
/// # Errors
/// A value that is not a number of seconds from 1 to 600.
pub fn timeout_from_env(raw: Option<&str>) -> Result<Duration, String> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(JOB_TIMEOUT),
        Some(text) => match text.parse::<u64>() {
            Ok(n) if (1..=600).contains(&n) => Ok(Duration::from_secs(n)),
            _ => Err(format!("{text:?} is not a number of seconds from 1 to 600")),
        },
    }
}

/// An account's slot among the running jobs, given back when dropped.
pub struct Slot<'a> {
    jobs: &'a Jobs,
    account: String,
}

impl Drop for Slot<'_> {
    fn drop(&mut self) {
        let mut per = self.jobs.per_account.lock();
        if let Some(n) = per.get_mut(&self.account) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                per.remove(&self.account);
            }
        }
    }
}

/// Why an ask was refused before any job started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// Past [`ASKS_PER_WINDOW`] or [`JOBS_PER_ACCOUNT`].
    TooMany,
    /// Past `BAYLEE_REPLAY_JOBS` running on this gateway.
    Busy,
}

/// Takes one of `account`'s asks and a slot to run it in.
///
/// # Errors
/// Past the account's budget or the gateway's.
pub fn admit<'a>(jobs: &'a Jobs, account: &str) -> Result<Slot<'a>, Refused> {
    let mut per = jobs.per_account.lock();
    let mine = per.get(account).copied().unwrap_or(0);
    if mine >= JOBS_PER_ACCOUNT {
        return Err(Refused::TooMany);
    }
    if per.values().map(|&n| n as usize).sum::<usize>() >= jobs.max {
        return Err(Refused::Busy);
    }
    if !jobs.asks.allow(account) {
        return Err(Refused::TooMany);
    }
    per.insert(account.to_owned(), mine + 1);
    drop(per);
    Ok(Slot {
        jobs,
        account: account.to_owned(),
    })
}

/// What a job asks of its engine.
pub struct Ask {
    /// The game.
    pub game_id: String,
    /// The seat the table is drawn from, or 0 for the public view.
    pub seat: u8,
    /// Seat, public, or all hands.
    pub perspective: v1::project_record::Perspective,
    /// Whether `seat`'s own hidden cards may be shown.
    pub own: bool,
    /// The other seats whose hands may be shown (only with `all_hands`).
    pub hands: Vec<u8>,
    /// The other seats whose hidden cards the drawn seat's own view may
    /// still show where it was shown them in the game.
    pub others: Vec<u8>,
    /// Whether to play on past a divergence.
    pub note: bool,
    /// The seats' names for the opening payload.
    pub names: Vec<String>,
}

/// The record of `ask.game_id`, played again by an engine an agent starts
/// for the job, as its tape (one gzip stream). The caller has checked that
/// the asking account sat at the game and the record's size.
///
/// # Errors
/// [`JobError`]: no agent with room, a held admission, no tape in time, a
/// tape too large, an engine that failed; or the database (`Err(None)`
/// is folded into [`JobError::Failed`]).
pub async fn run(state: &Shared, ask: Ask) -> Result<Vec<u8>, JobError> {
    if state.admission.held() {
        return Err(JobError::Held);
    }
    let record = match baylee_db::records::for_projection(&state.db, &ask.game_id).await {
        Ok(Some(record)) => record,
        Ok(None) => return Err(JobError::Failed),
        Err(e) => {
            tracing::error!("reading a record for a replay: {e}");
            return Err(JobError::Failed);
        }
    };
    let job_id = format!("{JOB_PREFIX}{}", auth::new_id());
    let started = Instant::now();
    let token = auth::new_token();
    let order = v1::ProjectRecord {
        job: job_id.clone(),
        record: record.data,
        perspective: ask.perspective as i32,
        own: ask.own,
        hands: ask.hands.iter().map(|&h| u32::from(h)).collect(),
        others: ask.others.iter().map(|&o| u32::from(o)).collect(),
        seat: u32::from(ask.seat),
        names: ask.names,
        verify: if ask.note {
            v1::project_record::Verify::Note
        } else {
            v1::project_record::Verify::Stop
        } as i32,
        game_id: ask.game_id.clone(),
        recorded_by: record.build.unwrap_or_default(),
    };
    let (done, tape) = oneshot::channel();
    let agent_id = {
        let mut agents = state.agents.lock();
        let Some(agent_id) = agents.pick() else {
            return Err(JobError::NoCapacity);
        };
        let start = Envelope {
            msg: Some(v1::envelope::Msg::StartEngine(v1::StartEngine {
                game_id: job_id.clone(),
                engine_token: token.clone(),
                gateway_url: state.engine_url.clone(),
            })),
        };
        let Some(agent) = agents.connected.get_mut(&agent_id) else {
            return Err(JobError::NoCapacity);
        };
        if agent.tx.send(start).is_err() {
            return Err(JobError::NoCapacity);
        }
        agent.games.push(job_id.clone());
        state.replay_jobs.running.lock().insert(
            job_id.clone(),
            Job {
                token_hash: auth::token_hash(&token),
                order: Some(order),
                done: Some(done),
            },
        );
        agent_id
    };
    let answer = match tokio::time::timeout(state.replay_jobs.timeout, tape).await {
        Ok(Ok(answer)) => answer,
        Ok(Err(_)) => Err(JobError::Failed),
        Err(_) => Err(JobError::Timeout),
    };
    forget(state, &job_id, &agent_id, answer.is_err());
    let outcome = match &answer {
        Ok(_) => "tape",
        Err(JobError::Timeout) => "timeout",
        Err(JobError::TooLarge) => "too large",
        Err(_) => "failed",
    };
    tracing::info!(
        job = job_id,
        game_id = ask.game_id,
        outcome,
        ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        "replay job"
    );
    answer
}

/// The job is over: out of the registry and off its agent's games, its
/// engine told to stop when it did not finish on its own.
fn forget(state: &Shared, job_id: &str, agent_id: &str, stop: bool) {
    state.replay_jobs.running.lock().remove(job_id);
    let mut agents = state.agents.lock();
    if stop {
        agents.tell(
            agent_id,
            Envelope {
                msg: Some(v1::envelope::Msg::StopEngine(v1::StopEngine {
                    game_id: job_id.to_owned(),
                })),
            },
        );
    }
    if let Some(agent) = agents.connected.get_mut(agent_id) {
        agent.games.retain(|g| g != job_id);
    }
}

/// Answers a job's waiter, if anybody still waits.
fn answer(state: &Shared, job_id: &str, result: Result<Vec<u8>, JobError>) {
    let done = state
        .replay_jobs
        .running
        .lock()
        .get_mut(job_id)
        .and_then(|job| job.done.take());
    if let Some(done) = done {
        let _ = done.send(result);
    }
}

/// An agent says a job's engine failed to start.
pub fn failed(state: &Shared, job_id: &str) {
    answer(state, job_id, Err(JobError::Failed));
}

/// The order a job's engine is sent after its hello, if `token` is the
/// job's; taken, so a second engine for the same job is sent nothing.
pub fn order_for(state: &Shared, job_id: &str, token: &str) -> Option<v1::ProjectRecord> {
    let mut running = state.replay_jobs.running.lock();
    let job = running.get_mut(job_id)?;
    if !auth::ct_eq(&job.token_hash, &auth::token_hash(token)) {
        return None;
    }
    job.order.take()
}

/// Gathers a job's tape from its engine's pieces: in `seq` order, up to
/// [`MAX_TAPE_BYTES`], until the `last`. Hands on a piece at a time; the
/// result is what the job's waiter is answered once the last arrives (or
/// the socket closes first).
pub struct Gather {
    job_id: String,
    tape: Vec<u8>,
    next: u32,
    failed: Option<JobError>,
}

impl Gather {
    /// For job `job_id`.
    #[must_use]
    pub fn new(job_id: &str) -> Self {
        Self {
            job_id: job_id.to_owned(),
            tape: Vec::new(),
            next: 0,
            failed: None,
        }
    }

    /// Takes one piece; `true` once the tape is whole (or past hope).
    pub fn take(&mut self, piece: &v1::TapeChunk) -> bool {
        if piece.job != self.job_id || piece.seq != self.next {
            self.failed = Some(JobError::Failed);
            return true;
        }
        self.next += 1;
        if self.tape.len().saturating_add(piece.data.len()) > MAX_TAPE_BYTES {
            self.failed = Some(JobError::TooLarge);
            return true;
        }
        self.tape.extend_from_slice(&piece.data);
        piece.last
    }

    /// The tape, or why there is none; `whole` says whether the last piece
    /// arrived.
    fn result(self, whole: bool) -> Result<Vec<u8>, JobError> {
        match self.failed {
            Some(e) => Err(e),
            None if whole => Ok(self.tape),
            None => Err(JobError::Failed),
        }
    }

    /// Answers the job's waiter with what was gathered.
    pub fn finish(self, state: &Shared, whole: bool) {
        let job_id = self.job_id.clone();
        answer(state, &job_id, self.result(whole));
    }
}

/// A deck's colour identity (CR 903.4) as `WUBRG` letters, its cards' and
/// its commanders' together, for the replay list. Card data, read off the
/// pool; the same letters `baylee_gamehost::replay::deck_colours` gives a
/// local record (this crate does not link gamehost).
#[must_use]
pub fn deck_colours(spec: &baylee_core::preset::SeatSpec) -> String {
    let identity = spec
        .deck
        .iter()
        .chain(&spec.commanders)
        .filter_map(|entry| baylee_cards::by_index(entry.card))
        .fold(baylee_core::color::ColorSet::EMPTY, |set, card| {
            set.union(card.color_identity)
        });
    identity
        .iter()
        .map(baylee_core::color::Color::symbol)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(seq: u32, bytes: usize, last: bool) -> v1::TapeChunk {
        v1::TapeChunk {
            job: "replay-1".into(),
            seq,
            data: vec![7; bytes],
            last,
        }
    }

    #[test]
    fn a_tape_is_gathered_in_order_to_its_last_piece() {
        let mut gather = Gather::new("replay-1");
        assert!(!gather.take(&piece(0, 3, false)));
        assert!(gather.take(&piece(1, 2, true)));
        assert_eq!(gather.result(true), Ok(vec![7; 5]));

        let mut gather = Gather::new("replay-1");
        assert!(gather.take(&piece(1, 3, false)), "out of order");
        assert_eq!(gather.result(false), Err(JobError::Failed));

        let mut gather = Gather::new("replay-1");
        assert!(!gather.take(&piece(0, MAX_TAPE_BYTES, false)));
        assert!(gather.take(&piece(1, 1, true)), "one byte past the bound");
        assert_eq!(gather.result(true), Err(JobError::TooLarge));

        let mut gather = Gather::new("replay-1");
        assert!(!gather.take(&piece(0, 3, false)));
        assert_eq!(gather.result(false), Err(JobError::Failed), "cut off");
    }

    #[test]
    fn an_account_runs_two_jobs_at_once_and_the_gateway_its_bound() {
        let jobs = Jobs::new(3, JOB_TIMEOUT);
        let a1 = admit(&jobs, "a").expect("first");
        let a2 = admit(&jobs, "a").expect("second");
        assert_eq!(admit(&jobs, "a").err(), Some(Refused::TooMany));
        let b1 = admit(&jobs, "b").expect("another account");
        assert_eq!(admit(&jobs, "c").err(), Some(Refused::Busy), "three run");
        drop(a1);
        let c1 = admit(&jobs, "c").expect("a slot given back");
        drop((a2, b1, c1));
        assert!(jobs.per_account.lock().is_empty());
        assert!(!Jobs::new(0, JOB_TIMEOUT).on());
    }

    #[test]
    fn an_account_asks_six_times_a_minute() {
        let jobs = Jobs::new(64, JOB_TIMEOUT);
        for i in 0..ASKS_PER_WINDOW {
            drop(admit(&jobs, "a").unwrap_or_else(|e| panic!("ask {i}: {e:?}")));
        }
        assert_eq!(admit(&jobs, "a").err(), Some(Refused::TooMany));
    }

    /// `source` with its `//` comments (doc comments too) taken out. A `//`
    /// inside a string literal is code and stays, as does a `'"'` char.
    fn code_of(source: &str) -> String {
        let mut out = String::with_capacity(source.len());
        for line in source.lines() {
            let bytes = line.as_bytes();
            let (mut in_string, mut escaped, mut end, mut i) = (false, false, line.len(), 0);
            while i < bytes.len() {
                let c = bytes[i];
                if in_string {
                    if escaped {
                        escaped = false;
                    } else if c == b'\\' {
                        escaped = true;
                    } else if c == b'"' {
                        in_string = false;
                    }
                } else if c == b'"' {
                    in_string = true;
                } else if bytes[i..].starts_with(b"'\"'") {
                    i += 2;
                } else if bytes[i..].starts_with(b"//") {
                    end = i;
                    break;
                }
                i += 1;
            }
            out.push_str(&line[..end]);
            out.push('\n');
        }
        out
    }

    /// How many times `source` calls `records::<function>(`, comments and
    /// docs not counted.
    fn call_sites(source: &str, function: &str) -> usize {
        code_of(source)
            .matches(&format!("records::{function}("))
            .count()
    }

    /// How many times `source` names `function` at all outside a comment:
    /// more than its call sites when it is imported, passed on as a value or
    /// re-exported, which is a second way to the same record.
    fn mentions(source: &str, function: &str) -> usize {
        code_of(source).matches(function).count()
    }

    /// Every `.rs` file under `dir`, by path.
    fn rust_files(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .map(|entry| entry.expect("a directory entry").path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                rust_files(&path, found);
            } else if path.extension().is_some_and(|e| e == "rs") {
                found.push(path);
            }
        }
    }

    #[test]
    fn the_scanner_counts_calls_and_not_mentions() {
        // Built from parts, so this file does not hold the names it looks
        // for outside a comment.
        let function = ["for", "seated"].join("_");
        let calls = format!(
            "fn a() {{ baylee_db::records::{function}(&db, 1); }}\n\
             fn b() {{ records::{function}(&db, 2) }}\n"
        );
        assert_eq!(call_sites(&calls, &function), 2);
        assert_eq!(mentions(&calls, &function), 2);

        let comments = format!(
            "//! A record goes out only by `records::{function}(` and a test.\n\
             /// `records::{function}(&db)` is called once.\n\
             fn a() {{}} // records::{function}(&db) in a trailing comment\n"
        );
        assert!(comments.contains(&format!("records::{function}(")));
        assert_eq!(call_sites(&comments, &function), 0);
        assert_eq!(mentions(&comments, &function), 0);

        // A `//` in a string is not a comment: the call after it counts.
        let string = format!("fn a() {{ let _ = \"a//b\"; records::{function}(&db) }}\n");
        assert_eq!(call_sites(&string, &function), 1);
        let ch = format!("fn a() {{ let _ = '\"'; records::{function}(&db) }} // x\n");
        assert_eq!(call_sites(&ch, &function), 1);

        // Imported, it is mentioned without being called as `records::f(`.
        let imported =
            format!("use baylee_db::records::{function};\nfn a() {{ {function}(&db) }}\n");
        assert_eq!(call_sites(&imported, &function), 0);
        assert_eq!(mentions(&imported, &function), 2);
    }

    /// A file's path under `src` and how often something is in it.
    type Counts = Vec<(String, usize)>;

    /// The files under `src` that call `records::<function>(`, and those
    /// that name `function` at all outside a comment, each with its count.
    fn callers(src: &std::path::Path, function: &str) -> (Counts, Counts) {
        let mut files = Vec::new();
        rust_files(src, &mut files);
        let (mut sites, mut named) = (Vec::new(), Vec::new());
        for path in &files {
            let source =
                std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let name = path
                .strip_prefix(src)
                .expect("under src")
                .to_string_lossy()
                .replace('\\', "/");
            let (calls, names) = (call_sites(&source, function), mentions(&source, function));
            if calls > 0 {
                sites.push((name.clone(), calls));
            }
            if names > 0 {
                named.push((name, names));
            }
        }
        (sites, named)
    }

    #[test]
    fn a_second_caller_in_another_file_is_found() {
        let function = ["for", "projection"].join("_");
        let dir = std::env::temp_dir().join(format!("baylee-callers-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("routes")).unwrap();
        let call = format!("fn a() {{ records::{function}(&db) }}\n");
        std::fs::write(dir.join("replays.rs"), &call).unwrap();
        std::fs::write(dir.join("routes/mod.rs"), "// records::x(\nfn b() {}\n").unwrap();
        let (sites, named) = callers(&dir, &function);
        assert_eq!(sites, [("replays.rs".to_owned(), 1)]);
        assert_eq!(named, sites);

        // A second call in the same file, a call in another, and an import.
        std::fs::write(dir.join("replays.rs"), call.repeat(2)).unwrap();
        std::fs::write(dir.join("routes/mod.rs"), &call).unwrap();
        std::fs::write(
            dir.join("routes/other.rs"),
            format!("use baylee_db::records::{function};\n"),
        )
        .unwrap();
        let (sites, named) = callers(&dir, &function);
        assert_eq!(
            sites,
            [
                ("replays.rs".to_owned(), 2),
                ("routes/mod.rs".to_owned(), 1)
            ]
        );
        assert_eq!(
            named,
            [
                ("replays.rs".to_owned(), 2),
                ("routes/mod.rs".to_owned(), 1),
                ("routes/other.rs".to_owned(), 1)
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A record leaves the store to a player's hand only through
    /// `POST /reports`, and to an engine only through a replay job
    /// (`docs/protocol.md` §"Replays"): one call site of each reader, in
    /// the one file that has the check before it.
    #[test]
    fn each_reader_of_a_whole_record_has_its_one_caller() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rust_files(&src, &mut files);
        // The population: the whole crate's sources, and not a stray few.
        assert!(
            (20..=500).contains(&files.len()),
            "{} files under src",
            files.len()
        );
        for (function, only) in [
            (["for", "seated"].join("_"), "report/mod.rs"),
            (["for", "projection"].join("_"), "replays.rs"),
        ] {
            let (sites, named) = callers(&src, &function);
            let expected = vec![(only.to_owned(), 1)];
            assert_eq!(sites, expected, "the call sites of {function}");
            assert_eq!(
                named, expected,
                "{function} is named outside a comment only at its call"
            );
        }
    }

    #[test]
    fn the_job_bound_reads_from_the_environment() {
        assert_eq!(jobs_from_env(None), Ok(DEFAULT_JOBS));
        assert_eq!(jobs_from_env(Some(" ")), Ok(DEFAULT_JOBS));
        assert_eq!(jobs_from_env(Some("0")), Ok(0));
        assert_eq!(jobs_from_env(Some("12")), Ok(12));
        assert!(jobs_from_env(Some("65")).is_err());
        assert!(jobs_from_env(Some("-1")).is_err());
        assert!(jobs_from_env(Some("four")).is_err());
        assert_eq!(timeout_from_env(None), Ok(JOB_TIMEOUT));
        assert_eq!(timeout_from_env(Some("5")), Ok(Duration::from_secs(5)));
        assert!(timeout_from_env(Some("0")).is_err());
        assert!(timeout_from_env(Some("601")).is_err());
    }
}
